# Retrieval

The language of the texts ragite stores and the passages it returns.

## Language

**Document**:
One input text, stored whole and cut into Chunks. A Document may contain no Chunks.
_Avoid_: file, article, page

**Document id**:
The positive integer ragite assigns to one Document when it is inserted. It is the Document table's `rowid`. A deleted id is not assigned again. Chunk ids use a separate sequence.
_Avoid_: source

**Format**:
The media type of a Document's stored text.
_Avoid_: extension, file type

**Chunk**:
A contiguous passage cut from one Document. It does not run across a section boundary. Its start and end name a span of the text it was cut from.
_Avoid_: passage, snippet, fragment, hit

**Heading path**:
The heading titles above a Chunk, from the outermost inward, stored as a JSON array of strings. An empty array when the Document is plain text.
_Avoid_: title, breadcrumb

**Score**:
The number the scoring algorithm assigns to a Chunk. A higher Score is a better match. A Document's Score is the Score of its best Chunk.
_Avoid_: distance, rank, weight

**Score column**:
The `score` column on a match result. It carries the Score for that row. It is null when the query is not a match.
_Avoid_: rank

**Source**:
An opaque label the application attaches to a Document, such as a path, a URL, or a title. More than one Document may carry the same Source. It may be empty.
_Avoid_: file, path, filename

**Hash**:
An opaque label stored with a Document. The caller may set it. When an insert omits it, ragite fills in the SHA-256 of the stored text.
_Avoid_: checksum, identity, key

**Table**:
A named collection of Documents in one database. A database may hold several Tables, and each Table keeps its own Chunks.
_Avoid_: corpus, index, collection

**Model**:
The embedding model bound to one Table. It is set when the Table is created and is read-only after that. Omitting it on create selects the build's default Model.
_Avoid_: weights, engine, backend

**Technical identifier**:
The model name joined to the revision of its bundled weights, as `name@revision`. Insert and embed use this identifier.
_Avoid_: default, alias

**Prefix**:
A string a Model adds in front of text before embedding. Each Model has a query prefix and a passage prefix, and either may be empty. The strings belong to the Model, not to the stored text.
_Avoid_: prompt, instruction, label
