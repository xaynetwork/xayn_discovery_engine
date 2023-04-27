// Copyright 2023 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

#[derive(Debug, PartialEq, Clone)]
struct Sentence {
    index: usize,
    length: usize,
    outgoing_connections: Option<HashMap<usize, usize>>,
    text: String,
    words: HashSet<String>,
    number_of_connections: f32,
}

#[derive(Debug, Clone)]
struct Summarizer<'a> {
    sentences: HashMap<usize, Sentence>,
    matrix: Option<Vec<Vec<f32>>>,
    bias_list: HashSet<String>,
    bias_strength: Option<f32>,
    _marker: std::marker::PhantomData<&'a str>,
}

pub(crate) fn summarize(text: &str, num_sentences: usize) -> String {
    let bias_strength = Some(2.0);
    let mut summariser = Summarizer::from_raw_text(text, ".", 50, 1500, false, bias_strength);
    let mut summary = summariser.top_sentences(
        num_sentences,
        false,
        None,
        false,
        5.0,
        false,
        3.0,
        None,
        bias_strength,
    );

    summary.sort_unstable_by(|a, b| a.index.partial_cmp(&b.index).unwrap());

    let mut sentence_indices = summariser.sentences.keys().copied().collect::<Vec<usize>>();

    sentence_indices.sort_unstable();

    summary
        .into_iter()
        .map(|s| s.text)
        .fold(String::new(), |mut acc, it| {
            acc.push_str(it.as_str());
            acc.push_str(".\n");
            acc
        })
}

impl<'a> Summarizer<'a> {
    fn from_raw_text(
        raw_text: &str,
        separator: &str,
        min_length: usize,
        max_length: usize,
        ngrams: bool,
        bias_strength: Option<f32>,
    ) -> Summarizer<'a> {
        let sentences = Arc::new(Mutex::new(HashMap::new()));
        let all_sentences = raw_text.split(separator).collect::<Vec<&str>>();

        for (i, sentence) in all_sentences.iter().enumerate() {
            if sentence.len() > min_length && sentence.len() < max_length {
                let mut words: HashSet<String> = HashSet::new();
                if ngrams {
                    for n in 7..15 {
                        let ngrams = sentence
                            .chars()
                            .collect::<Vec<char>>()
                            .windows(n)
                            .map(|x| x.iter().collect::<String>())
                            .collect::<Vec<String>>();
                        words.extend(ngrams);
                    }
                } else {
                    words = sentence
                        .split_whitespace()
                        .map(std::string::ToString::to_string)
                        .collect::<HashSet<_>>();
                }
                let outgoing_connections = HashMap::new();
                let sentence = Sentence {
                    words,
                    index: i,
                    length: sentence.len(),
                    outgoing_connections: Some(outgoing_connections),
                    text: (*sentence).to_string(),
                    number_of_connections: 0.0,
                };
                sentences.lock().unwrap().insert(i, sentence.clone());
            }
        }
        let final_sentences = sentences.lock().unwrap().clone();

        Summarizer {
            bias_strength,
            sentences: final_sentences,
            matrix: None,
            bias_list: HashSet::new(),
            _marker: std::marker::PhantomData,
        }
    }

    fn from_sentences_direct(sentences: HashMap<usize, Sentence>) -> Summarizer<'a> {
        Summarizer {
            sentences,
            matrix: None,
            bias_list: HashSet::new(),
            bias_strength: None,
            _marker: std::marker::PhantomData,
        }
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn top_sentences(
        &mut self,
        number_of_sentences_to_return: usize,
        return_summaries_for_each: bool,
        chunk_size: Option<usize>,
        force_sum_all: bool,
        length_penalty: f32,
        force_chunk: bool,
        density: f32,
        bias_list: Option<HashSet<String>>,
        bias_strength: Option<f32>,
    ) -> Vec<Sentence> {
        // If longer than 10,000, then divide it into portions of 5000 each. Instantiate new
        // Summarizers and call Summarizer::from_sentences_direct on each one, passing in the
        // portion of the original sentences (convert the HashMap to a vec). Then call
        // Summarizer::top_sentences on each one, passing in the number of sentences to return.
        // Collect the sentences, and pass them to a new instance of
        // Summarizer::from_sentences_direct. Then call Summarizer::top_sentences on that
        // instance, passing in the number of sentences to return. Return the result.
        if bias_list.is_some() {
            self.bias_list = bias_list.clone().unwrap();
        }
        if bias_strength.is_some() {
            self.bias_strength = bias_strength;
        } else {
            self.bias_strength = Some(2.0);
        }
        let length_of_sentences = self.sentences.len();
        if force_chunk
            || !force_sum_all && (length_of_sentences > 2000 || return_summaries_for_each)
        {
            //if chunk_size is specified, then use that. otherwise use a default value of 2000
            let final_chunk_size = chunk_size.unwrap_or(500);
            let mut summarisers = self
                .sentences
                .clone()
                .into_iter()
                .collect::<Vec<(usize, Sentence)>>()
                .chunks(final_chunk_size)
                .map(|chunk| {
                    let mut new_sentences = HashMap::new();
                    for (initial, (_, sentence)) in chunk.iter().enumerate() {
                        new_sentences.insert(initial, sentence.clone());
                    }
                    Summarizer::from_sentences_direct(new_sentences)
                })
                .collect::<Vec<Summarizer<'a>>>();
            let collected_sentences = summarisers
                .iter_mut()
                .map(|summariser| {
                    let indiv_num_to_return = if return_summaries_for_each {
                        number_of_sentences_to_return
                    } else {
                        //number_of_sentences_to_return * number_of_summarisers.clone(),

                        100
                    };
                    summariser.top_sentences(
                        indiv_num_to_return,
                        false,
                        None,
                        true,
                        length_penalty,
                        false,
                        density,
                        bias_list.clone(),
                        bias_strength,
                    )
                })
                .collect::<Vec<Vec<Sentence>>>();
            if return_summaries_for_each {
                let collected_sentences = collected_sentences
                    .into_iter()
                    .flatten()
                    .collect::<Vec<Sentence>>();
                return collected_sentences;
            }
            let collected_sentences = collected_sentences
                .into_iter()
                .flatten()
                .collect::<Vec<Sentence>>();
            let mut summariser = Summarizer::from_sentences_direct(
                collected_sentences
                    .into_iter()
                    .enumerate()
                    .map(|(index, sentence)| (index, sentence))
                    .collect::<HashMap<_, _>>(),
            );
            let final_sentences = summariser.top_sentences(
                number_of_sentences_to_return,
                false,
                None,
                false,
                length_penalty,
                false,
                density,
                bias_list,
                bias_strength,
            );
            return final_sentences;
        }

        let mut matrix = vec![vec![0.0; length_of_sentences]; length_of_sentences];

        matrix.iter_mut().enumerate().for_each(|(i, row)| {
            if let Some(sentence) = self.sentences.get(&i.clone()) {
                for (j, row_j) in row
                    .iter_mut()
                    .enumerate()
                    .take(length_of_sentences)
                    .skip(i + 1)
                {
                    #[allow(clippy::cast_precision_loss)]
                    if let Some(other) = self.sentences.get(&j) {
                        *row_j = (number_of_word_connections(sentence, other) as f32).powf(density)
                            / (sentence.length as f32).powf(length_penalty);
                    }
                }
            }
        });

        self.matrix = Some(matrix.clone());

        let mut top_sentences = matrix
            .iter()
            .enumerate()
            .map(|(i, row)| (row.iter().sum::<f32>(), i))
            .filter(|(sum, _)| *sum > 0.0)
            .collect::<Vec<(f32, usize)>>();
        top_sentences.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        top_sentences
            .iter()
            .take(number_of_sentences_to_return)
            .map(|x| x.1)
            .collect::<Vec<usize>>()
            .iter()
            .filter(|x| self.sentences.contains_key(x))
            .map(|x| self.sentences.get(x).unwrap().clone())
            .collect::<Vec<Sentence>>()
    }
}

fn number_of_word_connections(sentence: &Sentence, other: &Sentence) -> usize {
    sentence.words.intersection(&other.words).count()
}
