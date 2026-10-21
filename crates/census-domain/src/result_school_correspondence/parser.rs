use super::RESULT_SCHOOL_PARSER_CONTRACT_V1;

pub const RESULT_SCHOOL_TFRRS_PARSER_CONTRACT_V1: &[u8] = br#"{"version":1,"contract":"result-school-tfrrs-original-team-cell-v1","row_interpretation":"retained-tfrrs-list-row/v1","statement_interpretation":"retained-tfrrs-original-school-statement/v1","ownership":"unique-untransformed-original-team-anchor-text","identity_minting":"forbidden"}"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultSchoolSourceParserV1 {
    MileSplit,
    Tfrrs,
}

impl ResultSchoolSourceParserV1 {
    pub fn from_contract(bytes: &[u8]) -> Option<Self> {
        if bytes == RESULT_SCHOOL_PARSER_CONTRACT_V1 {
            Some(Self::MileSplit)
        } else if bytes == RESULT_SCHOOL_TFRRS_PARSER_CONTRACT_V1 {
            Some(Self::Tfrrs)
        } else {
            None
        }
    }

    pub fn contract(self) -> &'static [u8] {
        match self {
            Self::MileSplit => RESULT_SCHOOL_PARSER_CONTRACT_V1,
            Self::Tfrrs => RESULT_SCHOOL_TFRRS_PARSER_CONTRACT_V1,
        }
    }

    pub fn row_interpretation(self) -> &'static str {
        match self {
            Self::MileSplit => "retained-milesplit-html-result-row/v1",
            Self::Tfrrs => "retained-tfrrs-list-row/v1",
        }
    }

    pub fn document_namespace(self) -> &'static str {
        match self {
            Self::MileSplit => "milesplit_result_document_v1",
            Self::Tfrrs => "tfrrs_result_document_v1",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_parser_bytes_select_one_closed_row_and_namespace_contract(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for parser in [
            ResultSchoolSourceParserV1::MileSplit,
            ResultSchoolSourceParserV1::Tfrrs,
        ] {
            check!(eq; ResultSchoolSourceParserV1::from_contract(parser.contract()), Some(parser));
            let mut changed = parser.contract().to_vec();
            changed.push(b' ');
            check!(ResultSchoolSourceParserV1::from_contract(&changed).is_none());
        }
        check!(ne; ResultSchoolSourceParserV1::MileSplit.row_interpretation(), ResultSchoolSourceParserV1::Tfrrs.row_interpretation());
        check!(ne; ResultSchoolSourceParserV1::MileSplit.document_namespace(), ResultSchoolSourceParserV1::Tfrrs.document_namespace());
        check!(ResultSchoolSourceParserV1::from_contract(b"{}").is_none());
        Ok(())
    }
}
