use std::env;
use std::collections::HashMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::value::Value;
use yaml_rust::Yaml;
use tera::{Context, Tera, Result as TeraResult};

// Custom function based on tera
fn object(value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    Ok(value.unwrap().is_object())
}

fn get_env_hash() -> HashMap<String, String> {
    let mut env_vars = HashMap::new();
    for (key, value) in env::vars() {
        env_vars.insert(key, value);
    }
    return env_vars;
}

pub fn get_compiled_template_str_from_yaml(template_yaml_str: &Yaml) -> String {
    let template_str = match template_yaml_str.clone().into_string() {
        Some(t) => t,
        None => {
            println!("Failed to convert {:?} to string", template_yaml_str);
            ::std::process::exit(1); 
        },
    };
    return get_compiled_template_str(&template_str);
}

pub fn get_compiled_template_str(template: &String) -> String {
    let mut context = Context::new();
    context.insert("env", &get_env_hash());

    let result = match Tera::one_off(&template, &context, false) {
        Ok(s) => s,
        Err(e) => {
            println!("Could not compile template {:?}", template);
            println!("Error: {}", e);
            ::std::process::exit(1);
        }
    };
    return result;
}

pub fn get_compiled_template_str_with_context(template: &String, to_context: HashMap<String, String>) -> String {
    let mut context = Context::new();
    context.insert("env", &get_env_hash());
    context.insert("args", &to_context);

    let result = match Tera::one_off(&template, &context, false) {
        Ok(s) => s,
        Err(e) => {
            println!("Could not compile template {:?}", template);
            println!("Error: {}", e);
            ::std::process::exit(1);
        }
    };
    return result;
}

pub struct Template {
    tera: Tera,
}

impl Template {
    pub fn new() -> Template {
        let mut tera = match Tera::new("./templates/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Could not start tera: {}", e);
                ::std::process::exit(1);
            }
        };
        tera.register_tester("object", object);

        return Template {
            tera,
        }
    }

    pub fn get_compiled_template_with_context<T>(&mut self, template: String, context_hashes: HashMap<String, T>) -> String
    where T: DeserializeOwned, T:Serialize {
        let mut context = Context::new();
        for (key, value) in context_hashes.iter() {
            context.insert(&key, &value);
        }
        context.insert(&String::from("env"), &get_env_hash());

        let result = match self.tera.render(&template, &context) {
            Ok(s) => s,
            Err(e) => {
                println!("Could not render template {:?}", template);
                println!("Error: {}", e);
                ::std::process::exit(1);
            }
        };
        return result;
    }
}
