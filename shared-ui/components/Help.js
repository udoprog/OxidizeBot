import React from 'react';
import CommandGroup from './CommandGroup';

function hash(s) {
  let out = new Set();

  for (let e of s.split(/\s+/)) {
    e = e.toLowerCase().replace(/[\s!<>`]+/, '');

    if (e.length === 0) {
      continue;
    }

    out.add(e);
  }

  return out;
}

function matches(test, s) {
  s = hash(s);

  for (let value of test.values()) {
    if (!setAny(s.values(), s => s.startsWith(value))) {
      return false;
    }
  }

  return true;

  function setAny(values, f) {
    for (let value of values) {
      if (f(value)) {
        return true;
      }
    }

    return false;
  }
}

export default class Help extends React.Component {
  constructor(props) {
    super(props);

    let q = new URLSearchParams(this.props.location.search);

    this.state = {
      loading: true,
      groups: props.commands.groups,
      filter: q.get('q') || '',
      showAll: false,
    }
  }

  componentDidMount() {
    this.setState({loading: false});
  }

  filter(groups) {
    let filter = this.state.filter;

    if (filter === '') {
      return [];
    }

    if (filter.startsWith('!')) {
      groups = groups.map(g => {
        let commands = g.commands.filter(c => {
          return c.name.startsWith(filter);
        });

        let modified = commands.length != g.commands;
        return Object.assign({}, g, {commands, modified});
      });
    } else {
      let test = hash(filter);

      groups = groups.map(g => {
        let commands = g.commands.filter(c => {
          return matches(test, c.name);
        });

        let modified = commands.length != g.commands;
        return Object.assign({}, g, {commands, modified});
      });
    }

    return groups.filter(g => g.commands.length > 0);
  }

  changeFilter(filter) {
    var path = `${this.props.location.pathname}`;

    if (!!filter) {
      var search = new URLSearchParams(this.props.location.search);
      search.set('q', filter);
      path = `${path}?${search}`
    }

    this.props.history.replace(path);
    this.setState({filter, showAll: false});
  }

  prevent(e) {
    e.preventDefault();
    return false;
  }

  toggleShowAll() {
    this.setState({showAll: !this.state.showAll, filter: ''});
  }

  render() {
    let groups = [];

    if (this.state.showAll) {
      groups = this.state.groups;
    } else {
      groups = this.filter(this.state.groups);
    }

    let clear = null;

    if (this.state.filter !== '') {
      clear = <div className='input-group-append'>
        <button className='btn btn-primary' onClick={() => this.changeFilter('')}>Clear Filter</button>
      </div>;
    }

    let toggleShowButton = null;

    if (this.state.filter === '') {
      if (this.state.showAll) {
        toggleShowButton = <button className="btn btn-default btn-danger" onClick={() => this.toggleShowAll()}>
          Hide all documentation
        </button>;
      } else {
        toggleShowButton = <button className="btn btn-default btn-danger" onClick={() => this.toggleShowAll()}>
          Show all documentation
        </button>;
      }

      toggleShowButton = <div className="mt-3 mb-3">
        {toggleShowButton}
      </div>;
    }

    let groupsRender = null;

    if (this.state.filter !== '' && groups.length === 0) {
      groupsRender = <div className="alert alert-warning mt-3 mb-3">No documentation matching "{this.state.filter}"</div>;
    } else {
      groupsRender = groups.map(c => {
        return <CommandGroup key={c.name} {...c} />;
      });
    }

    return (
      <>
        <h2 className='page-title'>Command Help</h2>

        <div className='alert alert-info'>
          <b>Want to help expand this page?</b><br />

          You can do that by contributing to the <a href='https://github.com/udoprog/OxidizeBot/blob/master/shared/commands.toml'><code>commands.toml</code></a> file on Github!
        </div>

        <h4>Search:</h4>

        <form onSubmit={this.prevent.bind(this)}>
          <div className='input-group'>
            <input className='form-control' placeholder='filter' value={this.state.filter || ''} onChange={e => this.changeFilter(e.target.value)} />
            {clear}
          </div>
        </form>

        {toggleShowButton}

        {groupsRender}
      </>
    );
  }
}