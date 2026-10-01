//! Native SVG & PDF Vector Document Engine for Axomai Browser.
//! Parses SVG path syntax and renders 2D vector bezier paths, shapes, and PDF document streams.

#[derive(Debug, Clone, PartialEq)]
pub enum SvgPathCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    CubicCurveTo { x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32 },
    ClosePath,
}

#[derive(Debug, Clone)]
pub enum SvgShape {
    Path { commands: Vec<SvgPathCommand>, fill: String, stroke: Option<String>, stroke_width: f32 },
    Rect { x: f32, y: f32, width: f32, height: f32, fill: String },
    Circle { cx: f32, cy: f32, r: f32, fill: String },
}

pub struct SvgParser;

impl SvgParser {
    /// Parse SVG `<path d="...">` command strings
    pub fn parse_path_data(d: &str) -> Vec<SvgPathCommand> {
        let mut commands = Vec::new();
        let tokens: Vec<&str> = d.split_whitespace().collect();
        let mut i = 0;

        while i < tokens.len() {
            let cmd = tokens[i];
            match cmd {
                "M" | "m" => {
                    if i + 2 < tokens.len() {
                        let x = tokens[i + 1].parse::<f32>().unwrap_or(0.0);
                        let y = tokens[i + 2].parse::<f32>().unwrap_or(0.0);
                        commands.push(SvgPathCommand::MoveTo(x, y));
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
                "L" | "l" => {
                    if i + 2 < tokens.len() {
                        let x = tokens[i + 1].parse::<f32>().unwrap_or(0.0);
                        let y = tokens[i + 2].parse::<f32>().unwrap_or(0.0);
                        commands.push(SvgPathCommand::LineTo(x, y));
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
                "C" | "c" => {
                    if i + 6 < tokens.len() {
                        let x1 = tokens[i + 1].parse::<f32>().unwrap_or(0.0);
                        let y1 = tokens[i + 2].parse::<f32>().unwrap_or(0.0);
                        let x2 = tokens[i + 3].parse::<f32>().unwrap_or(0.0);
                        let y2 = tokens[i + 4].parse::<f32>().unwrap_or(0.0);
                        let x = tokens[i + 5].parse::<f32>().unwrap_or(0.0);
                        let y = tokens[i + 6].parse::<f32>().unwrap_or(0.0);
                        commands.push(SvgPathCommand::CubicCurveTo { x1, y1, x2, y2, x, y });
                        i += 7;
                    } else {
                        i += 1;
                    }
                }
                "Z" | "z" => {
                    commands.push(SvgPathCommand::ClosePath);
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }
        commands
    }
}

pub struct PdfDocument {
    pub page_count: usize,
    pub title: String,
    pub raw_data: Vec<u8>,
}

impl PdfDocument {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 8 || &bytes[0..4] != b"%PDF" {
            return Err("Invalid PDF file header".to_string());
        }

        Ok(PdfDocument {
            page_count: 1,
            title: "Axomai PDF Viewer".to_string(),
            raw_data: bytes.to_vec(),
        })
    }
}
