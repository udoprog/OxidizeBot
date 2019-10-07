import React from "react";
import ReactMarkdown from 'react-markdown';

function Content(props) {
  let source = props.source;

  if (source instanceof Array) {
    source = source.join('\n');
  }

  return <ReactMarkdown source={source} />
}

function ExampleContent(props) {
  let source = props.source;

  if (source instanceof Array) {
    source = source.join('\n');
  }

  return <pre><code>{source}</code></pre>
}

class Example extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    return <>
      <div className="example-name"><b>Example:</b> {this.props.name}</div>
      <div className="example-content"><ExampleContent source={this.props.content} /></div>
    </>;
  }
}

class Command extends React.Component {
  constructor(props) {
    console.log(props);
    super(props);
  }

  render() {
    let examples = null;

    if (this.props.examples && this.props.examples.length > 0) {
      examples = <>
        {(this.props.examples || []).map((e, i) => {
          return <Example key={i} {...e} />;
        })}
      </>;
    }

    return <>
      <div className="command">
        <div className="command-name"><code>{this.props.name}</code></div>
        <div className="command-content"><Content source={this.props.content} /></div>

        {examples}
      </div>
    </>;
  }
}

export class CommandGroup extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    let commands = null;

    if (this.props.commands && this.props.commands.length > 0) {
      commands = <>
        {(this.props.commands || []).map((c, i) => {
          return <Command key={i} {...c} />;
        })}
      </>;
    }

    return <>
      <div className="command-group">
        <div className="command-group-name">{this.props.name}</div>
        <div className="command-group-content"><ReactMarkdown source={this.props.content} /></div>

        {commands}
      </div>
    </>;
  }
}