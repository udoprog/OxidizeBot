import React from 'react';
import Example from './Example';
import { Header, Content } from '../utils';

export default class Command extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    let examples = null;

    if (this.props.examples && this.props.examples.length > 0) {
      examples = (this.props.examples || []).map((e, i) => {
        return <Example key={i} {...e} />;
      });
    }

    return <>
      <tr>
        <td className='oxi-command'>
          <div className='oxi-command-name'><Header source={this.props.name} /></div>
          <Content source={this.props.content} />
          {examples}
        </td>
      </tr>
    </>;
  }
}