use clap::Parser;
use std::process::{self, Command, Stdio};
use std::env;
use std::io::{self, Write};
use std::error::Error;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::path::Path;
use std::fs::File;
use std::io::copy;

const HOST: &str = "repository.plamolinux.org";
const MINIMUM_DIR_PATH: &str = "/pub/linux/Plamo/Plamo-8.x/x86_64/plamo/01_minimum/";
const DEVEL_DIR_PATH: &str = "/pub/linux/Plamo/Plamo-8.x/x86_64/plamo/02_devel/";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable auto installation
    #[arg(short, long, default_value = "false")]
    autoinstall: bool,    

    /// Enable download
    #[arg(short, long, default_value = "false")]
    download: bool,

    /// Continue even if errors occurs
    #[arg(short, long="continue", default_value = "false")]
    continue_get_pkginfo: bool,

    /// Download and install Python only
    #[arg(short, long, default_value = "false")]
    python: bool,

    /// Download and install get_pkginfo only
    #[arg(short, long, default_value = "false")]
    get_pkginfo: bool,

    /// Enable debug mode
    #[arg(long, default_value = "false")]
    debug: bool,

    /// Specify local blocks
    #[arg(short, long)]
    localblock: Vec<String>,

    /// Specify numbers in the range 0-16
    #[arg(default_value="")]
    categories: String,
}

fn parse_range(value: &str) -> Option<(i32, i32)> {
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() == 2 {
        if let (Ok(start), Ok(end)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
            if start >= 0 && end <= 16 && start <= end {
                return Some((start, end));
            }
        }
    }
    None
}

fn command_exist(command: &str) -> bool {
     Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)   
}

fn get_pkgname(host: &str, dir_path: &str, filename_ext: &str) -> Result<String, Box<dyn Error>> {
    let client = Client::new();

    // 引数からURLを作成してリクエストを作成
    let url = format!("http://{}{}", host, dir_path);
    let res = client.get(&url).send()?.text()?;

    // HTMLをパース
    let document = Html::parse_document(&res);

    // <a>タグを選択するセレクタ
    let selector = Selector::parse("a").unwrap();

    // セレクタで要素を抽出し、href属性を取得
    for element in document.select(&selector) {
        if let Some(filename) = element.value().attr("href") {
            let filename_start = format!("{}-", filename_ext);
            // ファイルリンクをフィルタリング
            if filename.starts_with(&filename_start) {
                return Ok(filename.to_string());
            }
        }
    }

    let error_str = format!("Cannot find {}", filename_ext);
    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, error_str)));
}

fn download_file(host: &str, dir_path: &str, filename: &str) -> Result<(), Box<dyn Error>> {
    let url = format!("http://{}{}{}", host, dir_path, filename);
    let client = Client::new();
    let mut response = client.get(&url).send()?;

    let mut dest = File::create(filename)?;
    copy(&mut response, &mut dest)?;

    Ok(())
}

fn yes_before_exec_command(command: &str, options: Vec<String>) -> Result<(), Box<dyn Error>> {
    // yesコマンドを呼び出し、その出力を取得
    let mut yes_process = Command::new("yes")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start yes command");

    // get_pkginfoコマンドを呼び出し、yesの出力をパイプで渡す
    let msg = format!("Failed to start {} command", command);
    let command_process = Command::new(command)
        .args(options)
        .stdin(yes_process.stdout.take().expect("Failed to capture stdout from yes"))
        .spawn()
        .expect(&msg);

    // yesプロセスが終了するまで待機
    let _ = yes_process.wait().expect("Failed to wait on yes");

    // get_pkginfoプロセスが終了するまで待機
    let msg = format!("Failed to start {} command", command);
    let output = command_process.wait_with_output().expect(&msg);

    // コマンドの終了コードを確認
    if !output.status.success() {
        let msg = format!("{} failed with error", command);
        eprintln!("{}", msg);
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg)))
    } else {
        // 成功した場合、標準出力を表示
        println!("{}", String::from_utf8_lossy(&output.stdout));
        Ok(())
    }
}

fn exec_command(command: &str, options: Vec<String>) -> Result<(), Box<dyn Error>> {
    // get_pkginfoコマンドを呼び出し、yesの出力をパイプで渡す
    let msg = format!("Failed to start {} command", command);
    let command_process = Command::new(command)
        .args(options)
        .spawn()
        .expect(&msg);

    // get_pkginfoプロセスが終了するまで待機
    let msg = format!("Failed to start {} command", command);
    let output = command_process.wait_with_output().expect(&msg);

    // コマンドの終了コードを確認
    if !output.status.success() {
        let msg = format!("{} failed with error", command);
        eprintln!("{}", msg);
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg)))
    } else {
        // 成功した場合、標準出力を表示
        println!("{}", String::from_utf8_lossy(&output.stdout));
        Ok(())
    }
}

fn main() {
    let args = Args::parse();
    let mut numbers = Vec::new();

    let categories: Vec<String> = args.categories.split_whitespace().map(String::from).collect();

    for category in categories {
        if let Some((start, end)) = parse_range(&category) {
            for i in start..=end {
                numbers.push(i);
            }
        }
        else if let Ok(num) = args.categories.parse::<i32>() {
            if num >= 0 && num <= 16 {
                numbers.push(num);
            }else{
                eprintln!("Error: Please specify number in the range 0-16: {}", args.categories);
                std::process::exit(1);
            }
        }
        else {
            eprintln!("Invalid categories: {}", args.categories);
            std::process::exit(1);
        }
    }

    // rootになるか確認
    if args.autoinstall || !command_exist("get_pkginfo") || !command_exist("python") {
        // 現在のユーザーIDを取得し、ルートかどうかをチェック
        let uid = unsafe { libc::getuid() };
        if uid != 0 {
            // プロンプトを表示して入力を取得
            print!("Do you want to install as root? [y/N] ");
            io::stdout().flush().unwrap(); // 標準出力をフラッシュして表示を確実にする

            let mut ans = String::new();
            io::stdin().read_line(&mut ans).expect("Failed to read input");

            // 入力をトリムしてチェック
            let ans = ans.trim();
            if ans.eq_ignore_ascii_case("y") {
                // 現在のプログラムを `su` コマンドで再実行
                let status = Command::new("su")
                    .arg("-c")
                    .arg(format!("{} {}", env::args().next().unwrap(), env::args().skip(1).collect::<Vec<_>>().join(" ")))
                    .status()
                    .expect("Failed to execute su command");

                // `su` が成功したかどうかをチェック
                if !status.success() {
                    eprintln!("Failed to gain root privileges.");
                    process::exit(1);
                }

                process::exit(status.code().unwrap_or(1));
            }
        }
    }

    // -aはroot以外には使用できない
    let uid = unsafe { libc::getuid() };
    if args.autoinstall {
        if uid != 0 {
            eprintln!("Please become root if you are using the -a/--autoinstall option");
            std::process::exit(1);
        }
    }

    if args.debug {
        print!("NUMBERS = ");
        for i in &numbers {
            print!("{} ", i); 
        }
        println!("");

        print!("LOCALBLOCK_ARGS = ");
        for i in &args.localblock {
            print!("{} ", i);
        }
        println!("");
    }

    if !command_exist("python") && !args.get_pkginfo {
        // Pythonのダウンロードとインストール
        let python_file_name;
        match get_pkgname(HOST, DEVEL_DIR_PATH, "Python") {
            Ok(name) => python_file_name = name,
            Err(e) => {
                eprintln!("Error: Cannot find python in repository: {}", e);
                std::process::exit(1);
            }
        }

        if Path::new(&python_file_name).exists() {
            println!("{} is already exists", python_file_name);
        }
        else {
            println!("Downloading Python package: {}", python_file_name);
            match download_file(HOST, DEVEL_DIR_PATH, &python_file_name) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        // rootであるか確認し、rootならばpythonをインストール
        if uid == 0 {
            if Path::new(&python_file_name).exists() {
                let status = Command::new("/sbin/installpkg")
                    .arg(&python_file_name)
                    .status()
                    .expect("Failed to execute installpkg command");

                if !status.success() {
                    eprintln!("Error: Failed to install python package");
                    std::process::exit(1);
                }
                else {
                    println!("Successfully installed python package");
                }
            }
        }
    }

    if !command_exist("get_pkginfo") && !args.python {
        // get_pkginfoのダウンロードとインストール
        let get_pkginfo_file_name;
        match get_pkgname(HOST, MINIMUM_DIR_PATH, "get_pkginfo") {
            Ok(name) => get_pkginfo_file_name = name,
            Err(e) => {
                eprintln!("Error: Cannot find get_pkginfo in repository: {}", e);
                std::process::exit(1);
            }
        }

        if Path::new(&get_pkginfo_file_name).exists() {
            println!("{} is already exists", get_pkginfo_file_name);
        }
        else {
            println!("Downloading get_pkginfo package: {}", get_pkginfo_file_name);
            match download_file(HOST, MINIMUM_DIR_PATH, &get_pkginfo_file_name) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        // rootであるか確認し、rootならばpythonをインストール
        if uid == 0 {
            if Path::new(&get_pkginfo_file_name).exists() {
                let status = Command::new("/sbin/installpkg")
                    .arg(&get_pkginfo_file_name)
                    .status()
                    .expect("Failed to execute installpkg command");

                if !status.success() {
                    eprintln!("Error: Failed to install python package");
                    std::process::exit(1);
                }
                else {
                    println!("Successfully installed python package");
                }
            }
        }
    }

    // Pythonかget_pkginfoが無いか、get_pkginfoまたはPythonだけをインストール指示された場合は終了
    if !command_exist("python") || !command_exist("get_pkginfo") || args.python || args.get_pkginfo {
        std::process::exit(0);
    }

    let mut options = String::new();

    if args.autoinstall {
        options += "-a ";
    }

    if args.download {
        options += "-d ";
    }

    if args.localblock.len() > 0 {
        options += "-l ";
        for i in &args.localblock {
            options += i;
            options += " ";
        }
    }

    if args.categories.len() == 0 {
        let options_vec: Vec<String> = options.split_whitespace().map(String::from).collect();
        if let Err(e) = yes_before_exec_command("get_pkginfo", options_vec) {
            if !args.continue_get_pkginfo {
                eprintln!("Error: get_pkginfo failed with error: {}", e); 
                std::process::exit(1);
            }
        }
    }

    for number in numbers {
        match number {
            0 => options += "-c 00_base ",
            1 => options += "-c 01_minimum ",
            2 => options += "-c 02_devel ",
            3 => options += "-c 03_libs ",
            4 => options += "-c 04_x11 ",
            5 => options += "-c 05_ext ",
            6 => options += "-c 06_xapps ",
            7 => options += "-c 07_multimedia ",
            8 => options += "-c 08_daemons ",
            9 => options += "-c 09_printings ",
            10 => options += "-c 10_xfce ",
            11 => options += "-c 11_lxqt ",
            12 => options += "-c 12_mate ",
            13 => options += "-c 13_tex ",
            14 => options += "-c 14_libreoffice ",
            15 => options += "-c 15_kernelsrc ",
            16 => options += "-c 16_virtualization ",
            _ => {
                eprintln!("Error: Invalid number: {}", number);
                std::process::exit(1);
            }
        }

        let options_vec: Vec<String> = options.split_whitespace().map(String::from).collect();
        if args.autoinstall {
            // -a/--autoinstallが指定されている場合はsudoを使用するかの確認にyesを使用
            if args.debug {
                println!("yes | get_pkginfo {}", options);
            }


            if let Err(e) = yes_before_exec_command("get_pkginfo", options_vec) {
                if !args.continue_get_pkginfo {
                    eprintln!("Error: get_pkginfo failed with error: {}", e); 
                    std::process::exit(1);
                }
            }
        }
        else {
            if args.debug {
                println!("get_pkginfo {}", options);
            }

            if let Err(e) = exec_command("get_pkginfo", options_vec) {
                if !args.continue_get_pkginfo {
                    eprintln!("Error: get_pkginfo failed with error: {}", e); 
                    std::process::exit(1);
                }
            }
        }
    }
}
