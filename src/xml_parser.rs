use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::BufRead;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
}

pub fn parse_xml<P: AsRef<Path>>(xml_path: P) -> Result<Vec<FileInfo>> {
    let xml_path = xml_path.as_ref();
    let file = std::fs::File::open(xml_path)
        .with_context(|| format!("Failed to open XML file: {}", xml_path.display()))?;
    let reader = std::io::BufReader::new(file);
    
    parse_xml_from_reader(reader)
}

fn parse_xml_from_reader<R: BufRead>(reader: R) -> Result<Vec<FileInfo>> {
    let mut xml_reader = Reader::from_reader(reader);
    xml_reader.trim_text(true);
    
    let mut buf = Vec::new();
    let mut files = Vec::new();
    let mut current_file: Option<FileInfo> = None;
    let mut current_tag = String::new();
    
    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                
                match current_tag.as_str() {
                    "name" => {
                        if current_file.is_none() {
                            current_file = Some(FileInfo {
                                name: text.clone(),
                                size: 0,
                            });
                        } else {
                            current_file.as_mut().unwrap().name = text;
                        }
                    }
                    "size" => {
                        if let Some(ref mut file) = current_file {
                            file.size = text.parse::<u64>()
                                .with_context(|| format!("Invalid size value: {}", text))?;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "file" || tag == "entry" {
                    if let Some(file) = current_file.take() {
                        if !file.name.is_empty() {
                            files.push(file);
                        }
                    }
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(anyhow::anyhow!("XML parsing error: {}", e));
            }
            _ => {}
        }
        buf.clear();
    }
    
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    
    #[test]
    fn test_parse_xml() {
        let xml_content = r#"
        <files>
            <file>
                <name>test1.txt</name>
                <size>1024</size>
            </file>
            <file>
                <name>test2.txt</name>
                <size>2048</size>
            </file>
        </files>
        "#;
        
        let reader = Cursor::new(xml_content);
        let files = parse_xml_from_reader(reader).unwrap();
        
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "test1.txt");
        assert_eq!(files[0].size, 1024);
        assert_eq!(files[1].name, "test2.txt");
        assert_eq!(files[1].size, 2048);
    }
}
