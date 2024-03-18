pub mod decoder;
pub mod node;
pub mod parser;
pub mod scope_manager;
pub mod tokenizer;
pub mod types;

mod tests {
    use crate::decoder::*;
    use crate::parser::*;
    use crate::scope_manager::*;
    use crate::tokenizer::*;
    use std::env;

    #[test]
    fn test() -> Result<(), String> {
        let input = r#"
    {

        l a = 2+2*(20-40);
        l b = a;
        l c = a+b;
    }
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
        let nodes = parser.program()?;
        println!("{:?}", nodes);
        let mut decoder = Decoder::new(&parser, &mut scope_manager);
        // 実行
        decoder.decode(&nodes)?;
        Ok(())
    }
}
