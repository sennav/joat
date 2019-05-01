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
use std::string::String;
use tera::{Context, Tera, Result as TeraResult};
use yaml_rust::Yaml;
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::vec::Vec;

fn get_string_from_yaml(yaml: &Yaml) -> String {
    match yaml.clone().into_string() {
        Some(s) => s,
        None => {
            println!("Failed to convert {:?} into string, exiting.", yaml);
            ::std::process::exit(1);
        },
    }
}

fn get_hash_from_yaml(yaml: &Yaml) -> HashMap<String, String> {
    let yaml_btree = match yaml.clone().into_hash() {
        Some(t) => t,
        None => {
            println!("Failed to convert to hash map, exiting.");
            ::std::process::exit(1);
        }
    };
    let mut yaml_hash = HashMap::new();
    for (key, value) in yaml_btree.iter() {
        let str_key = get_string_from_yaml(key);
        let parsed_value = get_compiled_template_str_from_yaml(value);
        yaml_hash.insert(str_key, parsed_value);
    };
    return yaml_hash;
}

fn get_subcommand_from_yaml(cmd_name: &str, yaml: &Yaml) -> Yaml {
    let subcommands = &yaml["subcommands"];
    let subcommands_vec = match subcommands.clone().into_vec() {
        Some(t) => t,
        None => {
            println!("Failed to retrieve subcommands, exiting.");
            ::std::process::exit(1);
        }
    };
    let cmd_name_yaml = Yaml::from_str(cmd_name);
    let scmd_option = subcommands_vec.iter().find(|&s| {
        match s.clone().into_hash() {
            Some(sl) => sl.contains_key(&cmd_name_yaml),
            None => false
        }
    });
    let scmd_hash = match scmd_option {
        Some(s) => s,
        None => {
            println!("Failed to retrieve subcommands hash, exiting.");
            ::std::process::exit(1);   
        }
    };
    return scmd_hash[cmd_name].clone();
}

fn get_compiled_template_str_from_yaml(template_yaml_str: &Yaml) -> String {
    let template_str = match template_yaml_str.clone().into_string() {
        Some(t) => t,
        None => {
            println!("Failed to convert {:?} to string", template_yaml_str);
            ::std::process::exit(1); 
        },
    };
    return get_compiled_template_str(&template_str);
}

// Custom function based on tera
fn object(value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    Ok(value.unwrap().is_object())
}

fn get_compiled_template_with_context<T>(template: String, context_hashes: HashMap<String, T>) -> String
    where T: DeserializeOwned, T:Serialize {
    let mut context = Context::new();
    for (key, value) in context_hashes.iter() {
        context.insert(&key, &value);
    }
    let mut tera = match Tera::new("./templates/*") {
        Ok(t) => t,
        Err(e) => {
            println!("Could not start tera: {}", e);
            ::std::process::exit(1);
        }
    };
    tera.register_tester("object", object);
    let result = match tera.render(&template, &context) {
        Ok(s) => s,
        Err(e) => {
            println!("Could not render template {:?}", template);
            println!("Error: {}", e);
            ::std::process::exit(1);
        }
    };
    return result;
}

fn get_compiled_template_str(template: &String) -> String {
    let mut context = Context::new();
    let mut env_vars = HashMap::new();
    for (key, value) in env::vars() {
        env_vars.insert(key, value);
    }
    context.insert("env", &env_vars);

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

fn get_compiled_template_str_with_context(template: &String, to_context: HashMap<String, String>) -> String {
    let mut env_vars = HashMap::new();
    for (key, value) in env::vars() {
        env_vars.insert(key, value);
    }

    let mut context = Context::new();
    context.insert("env", &env_vars);
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

fn get_env_hash() -> Value {
    let mut env_vars = Map::new();
    for (key, value) in env::vars() {
        let v: Value = value.into();
        env_vars.insert(key, v);
    }
    return Value::from(env_vars);
}

fn get_resource(endpoint: &String, headers: &HashMap<String, String>) -> Response {
    let client = reqwest::Client::new();
    let mut client_get = client.get(endpoint);
    for (name, value) in headers.iter() {
        client_get = client_get.header(&name[..], &value[..]);
    }
    let response = match client_get.send() {
        Ok(t) => t,
        Err(e) => {
            println!("Could not get response for endpoint {}", endpoint);
            println!("Error: {}", e);
            ::std::process::exit(1);
        }
    };
    return response;
}

fn get_complete_endpoint(base_endpoint: &Yaml, path_yaml: &Yaml) -> String {
    let mut endpoint = get_string_from_yaml(base_endpoint);
    let path_str = get_string_from_yaml(path_yaml);
    endpoint.push_str(&path_str);
    return endpoint;
}

fn get_param_split(param: String) -> (String, String) {
    let mut parts = param.split("=");
    if parts.clone().count() != 2 {
        println!("Params should be formatted as foo=bar");
        ::std::process::exit(1);
    }
    let raw_key = &String::from(parts.next().unwrap());
    let raw_value = &String::from(parts.next().unwrap());
    let key = get_compiled_template_str(raw_key);
    let value = get_compiled_template_str(raw_value);
    return (key, value);
}

fn stringify(query: Vec<(String, String)>) -> String {
    query.iter().fold(String::new(), |acc, tuple| {
        acc + &tuple.0 + "=" + &tuple.1 + "&"
    })
}

fn get_params(args: &ArgMatches) -> String {
    let mut param_vec = Vec::new();
    let params = match args.values_of("param") {
        Some(t) => t,
        None => {
            return String::from("");
        },
    };
    for p in params {
        let param = String::from(p);
        let splitted_param = get_param_split(param);
        param_vec.push(splitted_param);
    }
    return stringify(param_vec);
}

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

fn execute(cmd_name: &str, args: &ArgMatches, yaml: &Yaml) {
    let subcmd_yaml = get_subcommand_from_yaml(cmd_name, yaml);
    let raw_endpoint = get_complete_endpoint(&yaml["base_endpoint"], &subcmd_yaml["path"]);
    let params = get_params(args);
    let args = get_args_context(&args, &subcmd_yaml);
    let mut parsed_endpoint = get_compiled_template_str_with_context(&raw_endpoint, args);
    if params.len() > 0 {
        parsed_endpoint = format!("{}?{}", parsed_endpoint, params);
    }
    let headers = get_hash_from_yaml(&yaml["headers"]);
    let mut response = get_resource(&parsed_endpoint, &headers);
    let result: Value = match response.json() {
        Ok(r) => r,
        Err(e) => {
            println!("Could not convert response {:?} to json", response);
            println!("Error {:?}", e);
            ::std::process::exit(1);
        },
    };
    let mut response_context = HashMap::new();
    response_context.insert(String::from("response"), result);
    response_context.insert(String::from("env"), get_env_hash());
    let subcmd_hash = subcmd_yaml.clone().into_hash().unwrap();
    let mut template: String;
    if subcmd_hash.contains_key(&Yaml::from_str("template")) {
        template = subcmd_yaml["template"].clone().into_string().unwrap();
    } else {
        template = String::from("debug")
    }
    print!("{}", get_compiled_template_with_context(template, response_context));
}

fn main() {
    
    let yaml = load_yaml!("../cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    // let param = matches.value_of("param");
    // println!("Value for param: {:?}", param);

    match matches.subcommand() {
        (name, sub_cmd_option) => {
            match sub_cmd_option {
                Some(sub_cmd) => execute(name, sub_cmd, yaml),
                _ => ::std::process::exit(1)
            }
        }
    }

    // // Calling .unwrap() is safe here because "INPUT" is required (if "INPUT" wasn't
    // // required we could have used an 'if let' to conditionally get the value)

    // // Vary the output based on how many times the user used the "verbose" flag
    // // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    // match matches.occurrences_of("v") {
    //     0 => println!("No verbose info"),
    //     1 => println!("Some verbose info"),
    //     2 => println!("Tons of verbose info"),
    //     3 | _ => println!("Don't be crazy"),
    // }

    // // You can handle information about subcommands by requesting their matches by name
    // // (as below), requesting just the name used, or both at the same time
    // if let Some(matches) = matches.subcommand_matches("test") {
    //     if matches.is_present("debug") {
    //         println!("Printing debug info...");
    //     } else {
    //         println!("Printing normally...");
    //     }
    // }
    // println!("Yaml");
    // println!("{:?}", yaml);
    // println!("{:?}", yaml["subcommands"][0]["test"]["execute"]);

    // // more program logic goes here...
}