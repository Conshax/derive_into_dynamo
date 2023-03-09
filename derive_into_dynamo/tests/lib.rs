#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use aws_sdk_dynamodb;
    use derive_into_dynamo::IntoDynamoItem;

    type FakeUsize = usize;

    #[derive(IntoDynamoItem, Debug)]
    pub struct SubStruct {
        test: String,
    }

    #[derive(IntoDynamoItem, Debug)]
    pub struct TestStruct {
        string_name: String,
        usize_name: FakeUsize,
        isize_name: isize,
        bool_name: bool,
        vec_string_name: Vec<String>,
        option_name_some: Option<String>,
        option_name_none: Option<Vec<String>>,
        substruct_name: SubStruct,
        string_set_name: HashSet<String>,
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
            substruct_name: SubStruct {
                test: "substruct_string".to_string(),
            },
            string_set_name: HashSet::from_iter(["test_value".to_string()]),
        };

        let item = test.into_item();
        let test = TestStruct::from_item(item).unwrap();

        panic!("{test:?}")
    }
}
