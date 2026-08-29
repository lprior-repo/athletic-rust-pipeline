#[cfg(test)]
mod tests {
    use crate::alpha_config_test_helpers::valid_config;
    use crate::alpha_model::PaginationConfig;


    #[test]
    fn empty_complete_pointer_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::SingleResponse {
            complete_pointer: "".to_owned(),
        };
        let error = config.validate().expect_err("empty complete_pointer must fail");
        assert!(
            error.to_string().contains("complete_pointer"),
            "error: {}",
            error
        );
    }
    #[test]
    fn complete_pointer_trailing_tilde_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::SingleResponse {
            complete_pointer: "/settings/complete~".to_owned(),
        };
        let error = config.validate().expect_err("trailing ~ must fail");
        assert!(
            error.to_string().contains("invalid RFC6901"),
            "error: {}",
            error
        );
    }

    #[test]
    fn complete_pointer_invalid_escape_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::SingleResponse {
            complete_pointer: "/settings/complete~2".to_owned(),
        };
        let error = config.validate().expect_err("~2 escape must fail");
        assert!(
            error.to_string().contains("invalid RFC6901"),
            "error: {}",
            error
        );
    }

    #[test]
    fn has_more_pointer_invalid_escape_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::NextPage {
            has_more_pointer: "/data/~bad".to_owned(),
            next_page_pointer: "/data/next".to_owned(),
            request_page_key: "page".to_owned(),
        };
        let error = config.validate().expect_err("invalid escape in has_more must fail");
        assert!(
            error.to_string().contains("invalid RFC6901"),
            "error: {}",
            error
        );
    }

    #[test]
    fn next_page_pointer_invalid_escape_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::NextPage {
            has_more_pointer: "/data/hm".to_owned(),
            next_page_pointer: "/data/next~".to_owned(),
            request_page_key: "page".to_owned(),
        };
        let error = config.validate().expect_err("invalid escape in next_page must fail");
        assert!(
            error.to_string().contains("invalid RFC6901"),
            "error: {}",
            error
        );
    }
}
