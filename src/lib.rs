pub mod parser;
pub mod tokenizer;
pub mod types;

mod tests {
    use crate::parser::*;
    use crate::tokenizer::*;
    use std::env;

    #[test]
    fn test() -> Result<(), String> {
        let input = r#"
let a = 1919
let b = 1000
let d = 0
let c = a+b+
            "#;
        println!("ファイルの内容:\n{}", input);
        let temp_src = String::from(input);
        let src = temp_src.replace("\r", "");
        // トークナイズ
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(&src)?;
        let mut parser = Parser::new(&tokens);
        println!("tokens: {:?}", tokens);
        let mut scope_manager = ScopeManager::new();
        // パース
        let nodes = program(&mut parser)?;
        let mut decoder = Decoder::new(&parser, &mut scope_manager);
        // 実行
        decoder.decode(&nodes)?;
        Ok(())
    }
}
