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

use std::{cmp::min, collections::BTreeSet};

use ndarray::{Array1, Array2};
use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn summarize(text: &str, stop_words: &[&str], num_sentence: usize) -> String {
    let sentences = text.unicode_sentences().collect::<Vec<&str>>();
    if num_sentence >= sentences.len() {
        return text.to_string();
    }
    let mut sentences_and_words = vec![];
    for &sentence in &sentences {
        let words = split_into_words(sentence);
        sentences_and_words.push(words);
    }
    let matrix = build_similarity_matrix(&sentences_and_words, stop_words);
    let ranks = calculate_sentence_rank(&matrix);
    let mut sorted_ranks = ranks.clone();
    sorted_ranks.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let index = min(num_sentence + 1, sorted_ranks.len() - 1);
    let least_rank = sorted_ranks[index];
    let mut result: Vec<&str> = vec![];
    let mut included_count = 0;
    for i in 0..sentences.len() {
        if ranks[i] >= least_rank {
            included_count += 1;
            result.push(sentences[i]);
        }
        if included_count == num_sentence {
            break;
        }
    }
    result.join("")
}

fn get_all_words_lc<'a>(sentence1: &[&'a str], sentence2: &[&'a str]) -> BTreeSet<String> {
    sentence1
        .iter()
        .chain(sentence2.iter())
        .map(|w| w.to_lowercase())
        .collect()
}

/// Retrieve a sentence vector based on the frequency of words that appears in the `all_words_lc` set.
/// `all_words_lc` should be a sorted set of lower cased words
/// The size of the resulting vector is the same as the `all_words_lc` set
/// `stop_words` are skipped
fn get_sentence_vector(
    sentence: &[&str],
    all_words_lc: &BTreeSet<String>,
    stop_words: &[&str],
) -> Vec<usize> {
    let mut vector: Vec<usize> = vec![0; all_words_lc.len()];
    for word in sentence {
        let word_lc = word.to_lowercase();
        if !stop_words.contains(&word_lc.as_str()) {
            let index = all_words_lc.iter().position(|x| x.eq(&word_lc)).unwrap();
            vector[index] += 1;
        }
    }
    vector
}

/// Calculates the cosine distance between two vectors
/// Refer to [`YouTube`](https://www.youtube.com/watch?v=3X0wLRwU_Ws)
#[allow(clippy::cast_precision_loss)]
fn cosine_distance(vec1: &[usize], vec2: &[usize]) -> f64 {
    let dot_product = dot_product(vec1, vec2);
    let root_sum_square1 = l2_norm(vec1);
    let root_sum_square2 = l2_norm(vec2);
    dot_product as f64 / (root_sum_square1 * root_sum_square2)
}

#[allow(clippy::cast_precision_loss)]
fn l2_norm(vec: &[usize]) -> f64 {
    let sum_square = vec.iter().map(|x| x * x).sum::<usize>();
    (sum_square as f64).sqrt()
}

fn dot_product(vec1: &[usize], vec2: &[usize]) -> usize {
    vec1.iter().zip(vec2).map(|(a, b)| a * b).sum()
}

fn sentence_similarity(s1: &[&str], s2: &[&str], stop_words: &[&str]) -> f64 {
    let all_words = get_all_words_lc(s1, s2);
    let v1 = get_sentence_vector(s1, &all_words, stop_words);
    let v2 = get_sentence_vector(s2, &all_words, stop_words);
    1.0 - cosine_distance(&v1, &v2)
}

/// Calculate a similarity matrix for the given sentences.
/// Returns a 2-D array `M_i,j` such that for all 'j', sum(i, `M_i,j`) = 1
/// We take a leap of faith here and assume that cosine similarity is similar to the probability
/// that a sentence is important for summarization
fn build_similarity_matrix(sentences: &Vec<Vec<&str>>, stop_words: &[&str]) -> Array2<f64> {
    let len = sentences.len();
    let mut matrix = Array2::<f64>::zeros((len, len));
    let mut sum_column: Vec<f64> = vec![0.0; len];
    for i in 0..len {
        for j in 0..len {
            if i == j {
                continue;
            }
            matrix[[i, j]] =
                sentence_similarity(sentences[i].as_slice(), sentences[j].as_slice(), stop_words);
        }
    }
    // at this point we have the cosine similarity of each sentence.
    // take a leap of faith and assume that the cosine similarity is the probability that a sentence
    // is important for summarization.
    // We do this by normalizing the matrix along the column. The column values should add up to 1.
    for j in 0..len {
        let mut sum: f64 = 0.0;
        for i in 0..len {
            if i == j {
                continue;
            }
            sum += matrix[[i, j]];
        }
        sum_column[j] = sum;
    }
    for i in 0..len {
        for j in 0..len {
            if i == j {
                continue;
            }
            matrix[[i, j]] /= sum_column[j];
        }
    }
    matrix
}

/// Calculate a sentence rank similar to a page rank.
/// Please refer to [`PageRank`](https://en.wikipedia.org/wiki/PageRank) for more details.
fn calculate_sentence_rank(similarity_matrix: &Array2<f64>) -> Vec<f64> {
    let num_sentence = similarity_matrix.shape()[1];
    let threshold = 0.001;
    // Initialize a vector with the same value 1/number of sentences. Uniformly distributed across
    // all sentences. NOTE: perhaps we can make some sentences more important than the rest?
    #[allow(clippy::cast_precision_loss)]
    let initial_vector: Vec<f64> = vec![1.0 / num_sentence as f64; num_sentence];
    let mut result = Array1::from(initial_vector);
    let mut prev_result = result.clone();
    let damping_factor = 0.85;
    #[allow(clippy::cast_precision_loss)]
    let initial_m =
        damping_factor * similarity_matrix + (1.0 - damping_factor) / num_sentence as f64;
    loop {
        result = initial_m.dot(&result);
        let delta = &result - &prev_result;
        let mut converged = true;
        for i in 0..delta.len() {
            if delta[i] > threshold {
                converged = false;
                break;
            }
        }
        if converged {
            break;
        }
        prev_result = result.clone();
    }
    result.into_raw_vec()
}

fn split_into_words(sentence: &str) -> Vec<&str> {
    sentence.unicode_words().collect()
}
