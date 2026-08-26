#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitName {
    pub prefix: String,
    pub index: u32,
    pub count: u32,
}

/// Parses the conventional `name-00001-of-00004.gguf` shard suffix.
pub fn parse_split_filename(filename: &str) -> Option<SplitName> {
    let stem = filename
        .strip_suffix(".gguf")
        .or_else(|| filename.strip_suffix(".GGUF"))?;
    let (before_count, count_text) = stem.rsplit_once("-of-")?;
    let (prefix, index_text) = before_count.rsplit_once('-')?;
    if prefix.is_empty()
        || index_text.len() != 5
        || count_text.len() != 5
        || !index_text.bytes().all(|byte| byte.is_ascii_digit())
        || !count_text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }

    let index = index_text.parse().ok()?;
    let count = count_text.parse().ok()?;
    (count > 1 && index > 0 && index <= count).then(|| SplitName {
        prefix: prefix.to_string(),
        index,
        count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_the_llama_cpp_split_suffix() {
        assert_eq!(
            parse_split_filename("model-00001-of-00004.gguf"),
            Some(SplitName {
                prefix: "model".to_string(),
                index: 1,
                count: 4,
            })
        );
        assert_eq!(
            parse_split_filename("model-00004-of-00004.GGUF")
                .expect("uppercase extension")
                .index,
            4
        );
    }

    #[test]
    fn leaves_regular_or_malformed_names_alone() {
        for filename in [
            "model.gguf",
            "model-1-of-4.gguf",
            "model-00000-of-00004.gguf",
            "model-00005-of-00004.gguf",
            "model-00001-of-00001.gguf",
        ] {
            assert_eq!(parse_split_filename(filename), None, "{filename}");
        }
    }
}
