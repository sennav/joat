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
use std::collections::HashMap;
use yaml_rust::Yaml;
use std::process::Command;
use std::io::{self, Write};

mod template;
mod http;
mod yaml;

fn get_args_context(args: &ArgMatches, subcmd_yaml: &Yaml) -> HashMap<String, String> {
    let mut args_context = HashMap::new();
    for arg in subcmd_yaml["args"].clone().into_iter() {
        for a in arg.into_hash().unwrap().keys() {
            let key = a.clone().into_string().unwrap();
            if args.is_present(&key) {
                let value = args.value_of(&key).unwrap_or("");
                args_context.insert(key, String::from(value));
            }
        }
    }
    return args_context;
}

fn get_vars_context(yaml: &Yaml) -> HashMap<String, String> {
    let mut vars_context = HashMap::new();
    let vars_yaml = &yaml["vars"];
    if !vars_yaml.is_badvalue() {
        let vars_iter = vars_yaml
            .clone()
            .into_hash()
            .expect("Could not convert vars into hash");
        for (key, value) in vars_iter {
            let key_str = key.clone().into_string().expect("Var key should be string");
            let value_str = value.clone().into_string().expect("Var value should be string");
            vars_context.insert(key_str, value_str);
        }
    }
    return vars_context;
}

fn execute_script(context: HashMap<String, HashMap<String,String>>, subcmd_yaml: &Yaml) {
    let script_string = subcmd_yaml["script"].clone().into_string()
        .expect("Could not convert script to string");
    let script = template::get_compiled_template_str_with_context(
        &script_string,
        &context)
        .expect(format!("Could not parse script template {:?}", script_string).as_str());
    let output = Command::new("bash")
            .arg("-c")
            .arg(script)
            .output()
            .expect("failed to execute script");
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    assert!(output.status.success());
}

fn execute_request(cmd_name: &str, args: &ArgMatches, yaml: &Yaml, subcmd_yaml: &Yaml, context: HashMap<String, HashMap<String, String>>) {
    let subcmd_hash = subcmd_yaml.clone().into_hash().expect("Could not hash subcmd yaml");
    let mut http_method: String;
    if subcmd_hash.contains_key(&Yaml::from_str("method")) {
        http_method = subcmd_yaml["method"].clone().into_string().unwrap();
    } else {
        http_method = String::from("get")
    }

    let endpoint = http::get_endpoint(&cmd_name, &args, &context, &yaml);
    let headers = yaml::get_hash_from_yaml(&yaml["headers"], &context);
    let body = yaml::get_hash_from_yaml(&subcmd_yaml["body"], &context);

    let mut response = http::request(&http_method, &endpoint, &headers, &body);
    let result: Value = response.json().expect(&format!("Could not convert response {:?} to json", response));

    let mut response_context = HashMap::new();
    response_context.insert(String::from("response"), result);

    let mut template: String;
    if context["args"].contains_key("template") {
        template = context["args"]["template"].clone();
    } else if subcmd_hash.contains_key(&Yaml::from_str("template")) {
        template = subcmd_yaml["template"].clone().into_string().unwrap();
    } else {
        template = String::from("debug.j2")
    }
    let mut template_parser = template::Template::new(); // TODO remove mut
    print!("{}", template_parser.get_compiled_template_with_context(template, response_context));
}

fn execute(cmd_name: &str, args: &ArgMatches, yaml: &Yaml) {
    let subcmd_yaml = yaml::get_subcommand_from_yaml(cmd_name, yaml);
    let script = &subcmd_yaml["script"];

    let vars_context = get_vars_context(yaml);
    let args_context = get_args_context(&args, &subcmd_yaml);
    let mut context = HashMap::new();
    context.insert(String::from("vars"), vars_context);
    context.insert(String::from("args"), args_context);


    if !script.is_badvalue() {
        execute_script(context, &subcmd_yaml);
    } else {
        execute_request(&cmd_name, &args, &yaml, &subcmd_yaml, context);
    }
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