Valid States of the System
---

line_card:
	* count Integer
	* code String
	* set String

UrlVec = Vec<Uri>

single_face_card:
	* image_url String

multi_face_card:
	* image_urls URLVec

Obvious Errors:
	* MalformedFileError
	* MissingFileError
	* ParseJsonError
	* WebRequestError

