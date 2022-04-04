use crate::{
    coi::{point::UserInterests, RelevanceMap},
    data::{
        document::{DocumentHistory, DocumentId},
        document_data::{
            CoiComponent,
            DocumentDataWithCoi,
            DocumentDataWithContext,
            DocumentDataWithDocument,
            DocumentDataWithLtr,
            DocumentDataWithQAMBert,
            DocumentDataWithSMBert,
            SMBertComponent,
        },
    },
    error::Error,
};

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
#[allow(clippy::upper_case_acronyms)]
pub(crate) trait SMBertSystem {
    fn compute_embedding(
        &self,
        documents: &[DocumentDataWithDocument],
    ) -> Result<Vec<DocumentDataWithSMBert>, Error>;
}

pub(crate) trait CoiSystemData {
    fn id(&self) -> DocumentId;
    fn smbert(&self) -> &SMBertComponent;
    fn coi(&self) -> Option<&CoiComponent>;
}

#[cfg_attr(test, automock)]
pub(crate) trait CoiSystem {
    /// Add centre of interest information to a document
    fn compute_coi(
        &self,
        documents: &[DocumentDataWithSMBert],
        user_interests: &UserInterests,
    ) -> Result<Vec<DocumentDataWithCoi>, Error>;

    /// Update cois from history and documents
    fn update_user_interests<'a>(
        &mut self,
        history: &[DocumentHistory],
        documents: &[&'a dyn CoiSystemData],
        user_interests: UserInterests,
        relevances: &mut RelevanceMap,
    ) -> Result<UserInterests, Error>;
}

#[cfg_attr(test, automock)]
pub(crate) trait LtrSystem {
    fn compute_ltr(
        &self,
        history: &[DocumentHistory],
        documents: &[DocumentDataWithQAMBert],
    ) -> Result<Vec<DocumentDataWithLtr>, Error>;
}

#[cfg_attr(test, automock)]
pub(crate) trait ContextSystem {
    fn compute_context(
        &self,
        documents: Vec<DocumentDataWithLtr>,
    ) -> Result<Vec<DocumentDataWithContext>, Error>;
}

/// Common systems that we need in the reranker
/// At the moment this exists only to avoid to have 7+ generics around
pub(crate) trait CommonSystems {
    fn smbert(&self) -> &dyn SMBertSystem;
    fn coi(&self) -> &dyn CoiSystem;
    fn mut_coi(&mut self) -> &mut dyn CoiSystem;
    fn ltr(&self) -> &dyn LtrSystem;
    fn context(&self) -> &dyn ContextSystem;
}
