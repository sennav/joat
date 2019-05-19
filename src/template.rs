use std::env;
use std::collections::HashMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tera::{Context, Tera, Error };

fn get_env_hash() -> HashMap<String, String> {
    let mut env_vars = HashMap::new();
    for (key, value) in env::vars() {
        env_vars.insert(key, value);
    }
    return env_vars;
}

pub fn get_compiled_template_str(template: &String) -> String {
    let mut context = Context::new();
    context.insert("env", &get_env_hash());

    let result = match Tera::one_off(&template, context, false) {
        Ok(s) => s,
        Err(e) => {
            println!("Could not compile template {:?}", template);
            println!("Error: {}", e);
            ::std::process::exit(1);
        }
    };
    return result;
}

pub fn get_compiled_template_str_with_context(template: &String, raw_context: &HashMap<String, HashMap<String, String>>) -> Result<String, Error> {
    let mut context = Context::new();
    context.insert("env", &get_env_hash());
    for (k, v) in raw_context.iter() {
        context.insert(k, &v);
    }

    let result = Tera::one_off(&template, context, false)?;
    return Ok(result);
}

pub struct Template {
    tera: Tera,
}

impl Template {
    pub fn new(app_name: &str) -> Template {
        let mut tera = match Tera::new("./templates/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Could not start tera: {}", e);
                ::std::process::exit(1);
            }
        };
        let home_dir_path = match dirs::home_dir() {
            Some(h) => h,
            _ => {
                println!("Could not find home dir");
                ::std::process::exit(1);
            },
        };
        let home_dir_str = home_dir_path.into_os_string().into_string().unwrap();
        let home_path_str = String::from(format!("{}/.{}.joat/templates/**", home_dir_str, app_name));
        let tera_home_templates = Tera::new(home_path_str.as_str())
            .expect("Could not start Tera");
        tera.extend(&tera_home_templates).unwrap();

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

        let result = match self.tera.render(&template, context) {
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
