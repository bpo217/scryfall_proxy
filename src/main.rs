mod scryfall {
	use const_format::concatcp;

	const BEGIN_DOCUMENT_HTML: &'static str =
		concatcp!("<!DOCTYPE html>", "<html>", HEAD_HTML, "<body>", "<ul>");

	const END_DOCUMENT_HTML: &'static str =
		concatcp!("</ul>", "</body>", "</html>",);

	const HEAD_HTML: &'static str = concatcp!(
		"<head>",
		PRINTER_STYLE_HTML,
		"<title>Scryfall Proxy</title>",
		"</head>",
	);

	const PRINTER_STYLE_HTML: &'static str = indoc::indoc! {"
		<style>
			body {
				margin: 0;
				padding: 0;
				width: 210mm;
			}
			ul {
				align-content: flex-start;
				display: flex;
				flex-wrap: wrap;
				margin: 0;
				padding: 0;
				page-break-inside: avoid;
			}
			img {
				height: 88mm;
				width: 63mm;
			}
		</style>
	"};

	pub enum RuntimeError {
		InvalidCardCountNumberError,
		MalformedLineError,
		ParseJsonError,
		ParseStdinError,
		WebRequestBodyParseError,
		WebRequestError,
	}

	struct LineCard {
		count: u8,
		code: String,
		set: String,
	}

	#[derive(Clone, serde::Deserialize)]
	struct ImageUriGroup {
		large: String,
	}

	#[derive(serde::Deserialize, Clone)]
	struct CardFace {
		image_uris: ImageUriGroup,
	}

	#[derive(serde::Deserialize, Clone)]
	struct MultiCardFace {
		card_faces: Vec<CardFace>,
	}

	trait HtmlImgContent {
		fn img_content(&self) -> String;
	}

	type JsonString = String;

	impl LineCard {
		fn download(&self) -> Result<JsonString, RuntimeError> {
			let response = match reqwest::blocking::get(self.to_url()) {
				Ok(data) => data,
				Err(_) => return Err(RuntimeError::WebRequestError),
			};

			match response.text() {
				Ok(s) => Ok(s),
				Err(_) => Err(RuntimeError::WebRequestBodyParseError),
			}
		}

		fn parse_from(line_str: &str) -> Result<LineCard, RuntimeError> {
			let mut token_group = line_str.split(" ");
			let _count: u8 = match token_group.next() {
				Some(s) => match s.parse::<u8>() {
					Ok(n) => n,

					Err(_) => {
						return Err(RuntimeError::InvalidCardCountNumberError)
					}
				},

				None => return Err(RuntimeError::MalformedLineError),
			};

			let _code = match token_group.next() {
				Some(s) => s,
				None => return Err(RuntimeError::MalformedLineError),
			};

			let _set = match token_group.next() {
				Some(s) => s,
				None => return Err(RuntimeError::MalformedLineError),
			};

			Ok(LineCard {
				count: _count,
				code: _code.to_string(),
				set: _set.to_string(),
			})
		}

		fn to_url(&self) -> String {
			format!(
				"https://api.scryfall.com/cards/{}/{}",
				self.code, self.set
			)
		}
	}

	impl HtmlImgContent for CardFace {
		fn img_content(&self) -> String {
			format!("<li><img src=\"{}\"></li>", self.image_uris.large)
		}
	}

	impl HtmlImgContent for MultiCardFace {
		fn img_content(&self) -> String {
			self.card_faces
				.iter()
				.map(|card_face| card_face.img_content())
				.collect::<Vec<String>>()
				.join("")
		}
	}

	fn parse_json<T: for<'de> serde::Deserialize<'de> + Clone>(
		data: &JsonString
	) -> Result<T, RuntimeError> {
		match serde_json::from_str::<T>(&data) {
			Ok(card) => Ok(card),
			Err(_) => Err(RuntimeError::ParseJsonError),
		}
	}

	fn group_every_9(card_faces: &mut Vec<CardFace>) -> Vec<Vec<CardFace>> {
		let mut counter = 0;
		let mut outer_v: Vec<Vec<CardFace>> = vec![];
		let mut inner_v: Vec<CardFace> = vec![];

		for card_face in card_faces {
			inner_v.push(card_face.clone());

			if counter % 9 == 0 {
				outer_v.push(inner_v);
				inner_v = vec![];
			}

			counter += 1;
		}

		return outer_v;
	}

	pub fn exec() -> Result<String, RuntimeError> {
		let mut faces: Vec<CardFace> = vec![];
		let mut html = String::from("");

		for maybe_line in std::io::stdin().lines() {
			let line_card = match maybe_line {
				Ok(line) => LineCard::parse_from(&line)?,
				Err(_) => return Err(RuntimeError::ParseStdinError),
			};

			let json_data = line_card.download()?;

			if let Ok(card) = parse_json::<CardFace>(&json_data) {
				let mut allocated_faces: Vec<CardFace> =
					std::iter::repeat(card)
						.take(line_card.count as usize)
						.collect();
				faces.append(&mut allocated_faces);
			} else if let Ok(card) = parse_json::<MultiCardFace>(&json_data) {
				let mut allocated_multi_cardfaces: Vec<CardFace> =
					std::iter::repeat(card)
						.take(line_card.count as usize)
						.map(|multi_cardface| multi_cardface.card_faces)
						.flatten()
						.collect();
				faces.append(&mut allocated_multi_cardfaces);
			} else {
				return Err(RuntimeError::ParseJsonError);
			}
		}

		let body_html = group_every_9(&mut faces)
			.iter()
			.map(|card_faces| {
				card_faces
					.iter()
					.map(|card_face| card_face.img_content())
					.collect::<Vec<String>>()
					.join("")
			})
			.collect::<Vec<String>>()
			.join("</ul><ul>");

		html.push_str(BEGIN_DOCUMENT_HTML);
		html.push_str(&body_html);
		html.push_str(END_DOCUMENT_HTML);

		Ok(html)
	}
}

fn err_msg(e: scryfall::RuntimeError) -> &'static str {
	use scryfall::RuntimeError;
	match e {
		RuntimeError::InvalidCardCountNumberError => indoc::indoc! {"

			Input format per line is \"<x> <y> <z>\".
				• <x> is the card count in the deck.
				• <y> is the scryfall card set.
				• <z> is the scryfall card code.

			<x> failed to parse as a positive integer less than 256.
		"},
		RuntimeError::MalformedLineError => indoc::indoc! {"

				Input parse failed.
				Input format per line is <x> <y> <z>.
					• <x> is the card count in the deck.
					• <y> is the scryfall card set.
					• <z> is the scryfall card code.
		"},
		RuntimeError::ParseJsonError =>
			"JSON response parsing failed. However, the actual web request succeeded.",
		RuntimeError::ParseStdinError =>
			"STDIN failed to parse.",
		RuntimeError::WebRequestBodyParseError =>
			"Card web request downloaded successfully, but the body was malformed.",
		RuntimeError::WebRequestError =>
			"Card web request failed.",
	}
}

fn main() {
	match scryfall::exec() {
		Ok(s) => println!("{}", s),
		Err(e) => {
			eprintln!("{}", err_msg(e));
			std::process::exit(1);
		}
	}
}
