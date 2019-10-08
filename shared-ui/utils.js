import ReactMarkdown from 'react-markdown';

export function Header(props) {
  return <ReactMarkdown source={props.source} />
}

export function Content(props) {
  return <ReactMarkdown source={props.source} />
}

export function ExampleContent(props) {
  return <pre><code>{props.source}</code></pre>
}