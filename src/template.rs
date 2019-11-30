extern crate globwalk;

use crate::Context;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use tera::{Context as TeraContext, Error, Tera};

fn get_tera_context(context: &Context) -> TeraContext {
    let mut tera_context = TeraContext::new();
    for (k, v) in context.iter() {
        tera_context.insert(k, &v);
    }
    tera_context
}

pub fn get_compiled_template_str_with_context(
    template: &String,
    raw_context: &Context,
) -> Result<String, Error> {
    let context = get_tera_context(raw_context);

    let result = Tera::one_off(&template, context, false)?;
    return Ok(result);
}

pub struct Template {
    tera: Tera,
}

impl Template {
    pub fn new(app_name: &str) -> Template {
        let home_dir_path = dirs::home_dir().expect("Could not get home dir");

        // Add joat default templates
        let home_dir_str = home_dir_path
            .clone()
            .into_os_string()
            .into_string()
            .unwrap();
        let joat_path_str = String::from(format!("{}/.joat.joat/templates/**", home_dir_str));
        let mut tera = Tera::parse(joat_path_str.as_str()).expect("Could not start Tera");

        // Add local templates
        let templates_local = format!(".{}.joat/templates/", app_name);
        if Path::new(&templates_local).exists() {
            let templates_glob = format!(".{}.joat/templates/*.j2", app_name);
            let tera_local_templates =
                Tera::parse(&templates_glob).expect("Could not start tera with local templates");
            tera.extend(&tera_local_templates).unwrap();
        }

        // Add home tamplates
        let home_dir_str = home_dir_path.into_os_string().into_string().unwrap();
        let home_path_str =
            String::from(format!("{}/.{}.joat/templates/**", home_dir_str, app_name));
        let tera_home_templates =
            Tera::parse(home_path_str.as_str()).expect("Could not start Tera");
        tera.extend(&tera_home_templates).unwrap();

        tera.build_inheritance_chains().unwrap();

        return Template { tera };
    }

    pub fn get_compiled_template_with_context<T>(
        self,
        template: String,
        context_hashes: HashMap<String, T>,
    ) -> String
    where
        T: DeserializeOwned,
        T: Serialize,
    {
        let mut context = TeraContext::new();
        for (key, value) in context_hashes.iter() {
            context.insert(&key, &value);
        }

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
