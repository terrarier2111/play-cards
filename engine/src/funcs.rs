use crate::rt::RtRef;

pub fn println(args: Vec<RtRef>) -> Option<RtRef> {
    let val = args[0].get_string().unwrap().clone();
    let mut fmt = String::new();
    let mut split = val.split("{}");
    fmt.push_str(split.next().unwrap());
    for (idx, val) in split.enumerate() {
        fmt.push_str(&args[idx + 1].to_string());
        fmt.push_str(val);
    }
    println!("{}", fmt);
    None
}
