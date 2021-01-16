import React from 'react';
import Command from './Command';
import { Content } from '../utils';

export default class CommandGroup extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      expanded: false,
    };
  }

  toggle(expanded) {
    this.setState({expanded});
  }

  render() {
    let commands = null;

    let expand = this.state.expanded || !this.props.expandable || !!this.props.modified;

    if (this.props.commands && this.props.commands.length > 0 && expand) {
      commands = <table className='table table-dark table-striped'>
        <tbody>
          {(this.props.commands || []).map((c, i) => {
            return <Command key={i} {...c} />;
          })}
        </tbody>
      </table>;
    }

    let show = null;

    if (this.props.commands.length > 0 && !this.props.modified && this.props.expandable) {
      if (!this.state.expanded) {
        show = <button className='btn btn-info btn-sm' onClick={() => this.toggle(true)}>
          Show
        </button>;
      } else {
        show = <button className='btn btn-info btn-sm' onClick={() => this.toggle(false)}>
          Hide
        </button>;
      }
    }

    return <>
      <div className='oxi-command-group'>
        <div className='oxi-command-group-name'>
          {this.props.name}
        </div>

        <div className='oxi-command-group-content'><Content source={this.props.content} /></div>

        <div className='oxi-command-group-actions'>{show}</div>

        {commands}
      </div>
    </>;
  }
}
