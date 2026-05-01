use pest::Parser;
use pest_derive::Parser;
use crate::stages::{Stage, BuiltinStage, ScriptStage};

#[derive(Parser)] //ignore
#[grammar = "grammar/ril.pest"]
struct RilParser;


fn extract_string(pair: pest::iterators::Pair<Rule>) -> String {
    pair.into_inner()
        .find(|p| p.as_rule() == Rule::string)                                                                                                                                                            
        .unwrap()
        .into_inner()       // goes inside the string rule, past the quotes                                                                                                                               
        .as_str()                                                                                                                                                                                         
        .to_string()
}
fn extract_path(pair: pest::iterators::Pair<Rule>) ->  String {
    pair.into_inner()
        .find(|p| p.as_rule() == Rule::path)
        .unwrap()
        .as_str()
        .to_string()
}
fn extract_flags(pair: pest::iterators::Pair<Rule>) -> Vec<(String, String)> {
    pair.into_inner()
        .filter(|p| p.as_rule() == Rule::flag)                                                                                                                                                            
        .map(|flag| {                                                                                                                                                                                     
            let mut inner = flag.into_inner();
            let key = inner.next().unwrap().as_str().to_string();                                                                                                                                         
            let val = inner.next().unwrap().as_str().to_string();                                                                                                                                         
            (key, val)
        })                                                                                                                                                                                                
        .collect()
}

pub fn parse(input: &str) -> anyhow::Result<Vec<Stage>> {
    let pairs = RilParser::parse(Rule::pipeline, input)?;
    let mut stages = Vec::new();

    for pair in pairs.into_iter().next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::load => stages.push(Stage::Builtin(BuiltinStage::Load {
                path: extract_path(pair),
            })),
            Rule::save => stages.push(Stage::Builtin(BuiltinStage::Save {
                path: extract_path(pair),
            })),
            Rule::script => stages.push(Stage::Script(ScriptStage {
                path: extract_path(pair.clone()),
                flags: extract_flags(pair),
            })),
            // other main keywords and stuff
            _ => {}
        }
    }

    Ok(stages)
}