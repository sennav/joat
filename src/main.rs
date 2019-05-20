extern crate clap;
extern crate reqwest;
extern crate serde_json;
extern crate tera;
extern crate yaml_rust;
extern crate serde;
extern crate regex;
extern crate dirs;

use clap::App;
use clap::ArgMatches;
use serde_json::value::Value;
use std::collections::HashMap;
use yaml_rust::Yaml;
use std::process::Command;
use std::io::{self, Write};
use std::env;
use regex::Regex;
use std::fs;

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

fn execute_request(app_name: &String, cmd_name: &str, args: &ArgMatches, yaml: &Yaml, subcmd_yaml: &Yaml, context: HashMap<String, HashMap<String, String>>) {
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
    } else if subcmd_hash.contains_key(&Yaml::from_str("response_template")) {
        template = subcmd_yaml["response_template"].clone().into_string().unwrap();
    } else {
        template = String::from("debug.j2")
    }
    let mut template_parser = template::Template::new(app_name); // TODO remove mut
    print!("{}", template_parser.get_compiled_template_with_context(template, response_context));
}

fn execute_init(context: HashMap<String, HashMap<String, String>>) {
    let init_template = String::from(include_str!("../templates/config_template.yml"));
    let yaml_str = template::get_compiled_template_str_with_context(
        &init_template,
        &context)
        .expect("Could not create yaml template");
    let cmd_name = &context["args"]["PROJECT_NAME"];
    let filename = format!("{}.yml", cmd_name);
    fs::write(filename, yaml_str).expect("Unable to write file");
    println!("Config file {}.yml created", cmd_name);
    println!("To start testing with your extension create a symlink in your PATH targeting joat binaries with name: {}", cmd_name);
}

fn execute(app_name: &String, cmd_name: &str, args: &ArgMatches, yaml: &Yaml) {
    let subcmd_yaml = yaml::get_subcommand_from_yaml(cmd_name, yaml);
    let script = &subcmd_yaml["script"];

    let vars_context = get_vars_context(yaml);
    let args_context = get_args_context(&args, &subcmd_yaml);
    let mut context = HashMap::new();
    context.insert(String::from("vars"), vars_context);
    context.insert(String::from("args"), args_context);

    if app_name == "joat" && cmd_name == "init" {
        execute_init(context);
        return;
    }


    if !script.is_badvalue() {
        execute_script(context, &subcmd_yaml);
    } else {
        execute_request(&app_name, &cmd_name, &args, &yaml, &subcmd_yaml, context);
    }
}

fn install(cmd_name: &str, args: &ArgMatches, yaml: &Yaml) {
    println!("Install something");
}

fn format_cmd_name(cmd_name: &String) -> String {
    let re = Regex::new("[^/]*$").unwrap();
    String::from(
        re.find(cmd_name)
        .expect("Failed to parse main cmd name")
        .as_str()
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let app_name = format_cmd_name(&args[0]);
    let config_yaml = yaml::get_yaml_config(&app_name);
    let mut app = App::from_yaml(&config_yaml);
    let matches = app.clone().get_matches();
    match matches.subcommand() {
        (name, sub_cmd_option) => {
            match sub_cmd_option {
                Some(sub_cmd) => {
                    if name == "install" {
                        install(name, sub_cmd, &config_yaml)
                    } else {
                        execute(&app_name, name, sub_cmd, &config_yaml)
                    }
                },
                _ => {
                    // Could not find command, just print help
                    app.print_help().unwrap();
                    ::std::process::exit(1)
                }
            }
        }
    }
}