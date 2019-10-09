import React from 'react';
import { Header, ExampleContent } from '../utils';

export default class Example extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    return <>
      <div className='oxi-example-name'><b>Example:</b> <Header source={this.props.name} /></div>
      <div className='oxi-example-content'><ExampleContent source={this.props.content} /></div>
    </>;
  }
}