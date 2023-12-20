use crate::lsp::document::TypeDeclarationSearchInfo;
use crate::lsp::treesitter::ast_config::{AstConfig, Language};

pub struct HtmlConfig;

impl Language for HtmlConfig {
    fn make_ast_config() -> AstConfig {
        AstConfig {
            type_declaration_search_info: vec![
                TypeDeclarationSearchInfo::new("start_tag".to_string(), vec!["tag_name".to_string()]),
            ],
            namespace_search_info: None,
            keywords: vec![
                "<!–…–>", "<!DOCTYPE>", "<a>", "<abbr>", "<acronym>", "<address>", "<applet>", "<area>", "<article>",
                "<aside>", "<audio>", "<b>", "<base>", "<basefont>", "<bdi>", "<bdo>", "<big>", "<blockquote>", "<body>",
                "<br>", "<button>", "<canvas>", "<caption>", "<center>", "<cite>", "<code>", "<col>", "<colgroup>",
                "<data>", "<datalist>", "<dd>", "<del>", "<details>", "<dfn>", "<dialog>", "<dir>", "<div>", "<dl>", "<dt>",
                "<em>", "<embed>", "<fieldset>", "<figcaption>", "<figure>", "<font>", "<footer>", "<form>", "<frame>",
                "<frameset>", "<h1>", "<h2>", "<h3>", "<h4>", "<h5>", "<h6>", "<head>", "<header>", "<hr>", "<html>", "<i>",
                "<iframe>", "<img>", "<input>", "<ins>", "<kbd>", "<label>", "<legend>", "<li>", "<link>", "<main>",
                "<map>",
                "<mark>", "<meta>", "<meter>", "<nav>", "<noframes>", "<noscript>", "<object>", "<ol>", "<optgroup>",
                "<option>",
                "<output>", "<p>", "<param>", "<picture>", "<pre>", "<progress>", "<q>", "<rp>", "<rt>", "<ruby>", "<s>",
                "<samp>", "<script>", "<section>", "<select>", "<small>", "<source>", "<span>", "<strike>", "<strong>",
                "<style>", "<sub>", "<summary>", "<sup>", "<svg>", "<table>", "<tbody>", "<td>", "<template>", "<textarea>",
                "<tfoot>", "<th>", "<thead>", "<time>", "<title>", "<tr>", "<track>", "<tt>", "<u>", "<ul>", "<var>",
                "<video>", "<wbr>"
            ].iter().map(|s| s.to_string()).collect(),
            keywords_types: vec![].iter().map(|s: &&str| s.to_string()).collect(),
        }
    }
}