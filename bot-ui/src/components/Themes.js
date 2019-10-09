import React from "react";
import {Button, Alert, Table} from "react-bootstrap";
import {Loading, Error} from 'shared-ui/components';

function trackUrl(trackId) {
  if (trackId.startsWith("spotify:track:")) {
    let id = trackId.split(":")[2];
    return `https://open.spotify.com/track/${id}`;
  }

  if (trackId.startsWith("youtube:video:")) {
    let id = trackId.split(":")[2];
    return `https://youtu.be/${id}`;
  }

  return null;
}

export default class Themes extends React.Component {
  constructor(props) {
    super(props);

    this.api = this.props.api;

    this.state = {
      loading: true,
      error: null,
      data: null,
    };
  }

  async componentDidMount() {
    await this.list();
  }

  /**
   * Refresh the list of after streams.
   */
  async list() {
    this.setState({
      loading: true,
    });

    try {
      let data = await this.api.themes(this.props.current.channel);

      this.setState({
        loading: false,
        error: null,
        data,
      });
    } catch(e) {
      this.setState({
        loading: false,
        error: `failed to request after streams: ${e}`,
        data: null,
      });
    }
  }

  async editDisabled(key, disabled) {
    this.setState({
      loading: true,
      error: null,
    });

    try {
      await this.api.themesEditDisabled(key, disabled);
      await this.list();
    } catch(e) {
      this.setState({
        loading: false,
        error: `Failed to set disabled state: ${e}`,
      });
    }
  }

  render() {
    let loading = null;
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
                <th className="table-fill">Track ID</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {this.state.data.map((c, id) => {
                let disabled = null;

                if (c.disabled) {
                  let onClick = _ => {
                    this.editDisabled(c.key, false);
                  };
                  disabled = <Button className="button-fill" size="sm" variant="danger" onClick={onClick}>Disabled</Button>;
                } else {
                  let onClick = _ => {
                    this.editDisabled(c.key, true);
                  };
                  disabled = <Button className="button-fill" size="sm" variant="success" onClick={onClick}>Enabled</Button>;
                }

                let track = c.track_id;

                let url = trackUrl(c.track_id);

                if (!!url) {
                  track = <a href={url} target="track">{c.track_id}</a>;
                }

                return (
                  <tr key={id}>
                    <td className="theme-name">{c.key.name}</td>
                    <td className="theme-group"><b>{c.group}</b></td>
                    <td className="theme-start">{c.start}</td>
                    <td className="theme-end">{c.end}</td>
                    <td className="theme-track-id">{track}</td>
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
        <h1 className="oxi-page-title">Themes</h1>
        <Loading isLoading={this.state.loading} />
        <Error error={this.state.error} />
        {content}
        {loading}
      </div>
    );
  }
}