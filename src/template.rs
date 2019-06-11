extern crate globwalk;

use std::env;
use std::path::Path;
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
    template_path: HashMap<String, String>,
}

impl Template {
    pub fn new(app_name: &str) -> Template {
        let mut template_path = HashMap::new();

        let home_dir_path = dirs::home_dir().expect("Could not get home dir");

        // Add joat default templates
        let home_dir_str = home_dir_path.clone().into_os_string().into_string().unwrap();
        let joat_path_str = String::from(format!("{}/.joat.joat/templates/**", home_dir_str));
        let mut tera = Tera::parse(joat_path_str.as_str())
            .expect("Could not start Tera");

        for template in globwalk::glob(joat_path_str.clone()).unwrap() {
            if let Ok(template) = template {
                let filename = String::from(template.file_name().to_str().unwrap());
                template_path.insert(filename, joat_path_str.clone());
            }
        }

        if Path::new("./templates/").exists() {
            let tera_local_templates = Tera::parse("./templates/**")
                .expect("Could not start tera with local templates");
            for template in globwalk::glob("templates/**").unwrap() {
                if let Ok(template) = template {
                    let filename = String::from(template.file_name().to_str().unwrap());
                    template_path.insert(filename, String::from("./templates/"));
                }
            }
            tera.extend(&tera_local_templates).unwrap();
        }

        // Add home tamplates
        let home_dir_str = home_dir_path.into_os_string().into_string().unwrap();
        let home_path_str = String::from(format!("{}/.{}.joat/templates/**", home_dir_str, app_name));
        let tera_home_templates = Tera::parse(home_path_str.as_str())
            .expect("Could not start Tera");
        tera.extend(&tera_home_templates).unwrap();

        for template in globwalk::glob(home_path_str.clone()).unwrap() {
            if let Ok(template) = template {
                let filename = String::from(template.file_name().to_str().unwrap());
                template_path.insert(filename, home_path_str.clone());
            }
        }


        tera.build_inheritance_chains().unwrap();

        return Template {
            tera,
            template_path,
        }
    }

    pub fn get_compiled_template_with_context<T>(&mut self, template: String, context_hashes: HashMap<String, T>) -> String
    where T: DeserializeOwned, T:Serialize {
        let mut context = Context::new();
        for (key, value) in context_hashes.iter() {
            context.insert(&key, &value);
        }
        context.insert(&String::from("env"), &get_env_hash());

        let mut template_vars = HashMap::new();
        template_vars.insert(String::from("name"), &template);
        match self.template_path.get(&template) {
            Some(p) => { template_vars.insert(String::from("path"), p); },
            None => ()
        }
        context.insert(&String::from("template"), &template_vars);

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
