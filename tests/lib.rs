#[cfg(test)]
mod tests {
    use dynamo_parser::DynamoItem;

    type FakeUsize = usize;

    #[derive(DynamoItem)]
    pub struct TestStruct {
        string_name: String,
        usize_name: usize,
        isize_name: isize,
        bool_name: bool,
        vec_string_name: Vec<String>,
        option_name_some: Option<String>,
        option_name_none: Option<Vec<String>>,
    }

    #[test]
    fn it_works() {
        let test = TestStruct {
            string_name: "test_value".to_string(),
            usize_name: 25,
            isize_name: -5000,
            bool_name: true,
            vec_string_name: vec!["test_value".to_string(), "test_value2".to_string()],
            option_name_some: Some("x".to_string()),
            option_name_none: None,
        };

        let item = test.into_dynamo_item();

        panic!("{item:?}")
    }
}
