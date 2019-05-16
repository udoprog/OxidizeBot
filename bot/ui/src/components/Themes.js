import {Spinner} from "../utils.js";
import React from "react";
import {Button, Alert, Table} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";

export default class Themes extends React.Component {
  constructor(props) {
    super(props);

    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
      data: null,
    };
  }

  componentWillMount() {
    if (this.state.loading) {
      return;
    }

    this.list()
  }

  /**
   * Refresh the list of after streams.
   */
  list() {
    this.setState({
      loading: true,
    });

    this.api.themes(this.props.current.channel)
      .then(data => {
        this.setState({
          loading: false,
          error: null,
          data,
        });
      },
      e => {
        this.setState({
          loading: false,
          error: `failed to request after streams: ${e}`,
          data: null,
        });
      });
  }

  editDisabled(key, disabled) {
    this.setState({
      loading: true,
      error: null,
    });

    this.api.themesEditDisabled(key, disabled).then(_ => {
      return this.list();
    }, e => {
      this.setState({
        loading: false,
        error: `Failed to set disabled state: ${e}`,
      });
    });
  }

  render() {
    let error = null;

    if (this.state.error) {
      error = <Alert variant="warning">{this.state.error}</Alert>;
    }

    let refresh = null;
    let loading = null;

    if (this.state.loading) {
      loading = <Spinner />;
      refresh = <FontAwesomeIcon icon="sync" className="title-refresh right" />;
    } else {
      refresh = <FontAwesomeIcon icon="sync" className="title-refresh clickable right" onClick={() => this.list()} />;
    }

    let content = null;

    if (this.state.data) {
      if (this.state.data.length === 0) {
        content = (
          <Alert variant="info">
            No themes!
          </Alert>
        );
      } else {
        content = (
          <Table responsive="sm">
            <thead>
              <tr>
                <th>Name</th>
                <th>Group</th>
                <th>Start</th>
                <th>End</th>
                <th width="99%"></th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {this.state.data.map((c, id) => {
                let disabled = null;

                if (c.disabled) {
                  disabled = <Button size="sm" variant="danger" onClick={_ => {
                    this.editDisabled(c.key, false);
                  }}>Disabled</Button>;
                } else {
                  disabled = <Button size="sm" variant="success" onClick={_ => {
                    this.editDisabled(c.key, true);
                  }}>Enabled</Button>;
                }

                return (
                  <tr key={id}>
                    <td className="theme-name">{c.key.name}</td>
                    <td className="theme-group"><b>{c.group}</b></td>
                    <td className="theme-start">{c.start}</td>
                    <td className="theme-end">{c.end}</td>
                    <td></td>
                    <td>{disabled}</td>
                  </tr>
                );
              })}
            </tbody>
          </Table>
        );
      }
    }

    return (
      <div>
        <h2>
          Themes
          {refresh}
        </h2>
        {error}
        {content}
        {loading}
      </div>
    );
  }
}