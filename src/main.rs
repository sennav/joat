#[macro_use]
extern crate clap;
extern crate reqwest;
extern crate serde_json;
extern crate tera;
extern crate yaml_rust;
extern crate serde;

use clap::App;
use clap::ArgMatches;
use serde_json::value::Value;
use serde_json::map::Map;
use std::collections::HashMap;
use std::env;
use yaml_rust::Yaml;

mod template;
mod http;
mod yaml;

fn execute(cmd_name: &str, args: &ArgMatches, yaml: &Yaml) {
    let endpoint = http::get_endpoint(&cmd_name, &args, &yaml);
    let headers = yaml::get_hash_from_yaml(&yaml["headers"]);
    let mut response = http::get_resource(&endpoint, &headers);
    let result: Value = response.json().expect(&format!("Could not convert response {:?} to json", response));

    let mut response_context = HashMap::new();
    response_context.insert(String::from("response"), result);

    let mut template: String;
    let subcmd_yaml = yaml::get_subcommand_from_yaml(cmd_name, yaml);
    let subcmd_hash = subcmd_yaml.clone().into_hash().expect("Could not hash subcmd yaml");
    if subcmd_hash.contains_key(&Yaml::from_str("template")) {
        template = subcmd_yaml["template"].clone().into_string().unwrap();
    } else {
        template = String::from("debug")
    }
    let mut template_parser = template::Template::new(); // TODO remove mut
    print!("{}", template_parser.get_compiled_template_with_context(template, response_context));
}

fn main() {
    let yaml = load_yaml!("../cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    match matches.subcommand() {
        (name, sub_cmd_option) => {
            match sub_cmd_option {
                Some(sub_cmd) => execute(name, sub_cmd, yaml),
                _ => ::std::process::exit(1)
            }
        }
    }
}