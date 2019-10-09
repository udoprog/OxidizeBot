import React from "react";
import {Alert, Table} from "react-bootstrap";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import Loading from 'shared-ui/components/Loading';

export default class Authentication extends React.Component {
  constructor(props) {
    super(props);
    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
      data: null,
    };
  }

  async componentDidMount() {
    await this.listDevices();
  }

  /**
   * Refresh the list of devices.
   */
  async listDevices() {
    this.setState({
      loading: true,
    });

    try {
      let data = await this.api.devices();

      this.setState({
        loading: false,
        error: null,
        data,
      });
    } catch (e) {
      this.setState({
        loading: false,
        error: `failed to request devices: ${e}`,
        data: null,
      });
    }
  }

  /**
   * Pick the specified device.
   *
   * @param {string} id the device to pick.
   */
  async pickDevice(id) {
    this.setState({
      loading: true,
    });

    try {
      await this.api.setDevice(id);
    } catch(e) {
      this.setState({
        loading: false,
        error: `failed to pick device: ${e}`,
      });
    }
  }

  render() {
    let error = null;

    if (this.state.error) {
      error = <Alert variant="warning">{this.state.error}</Alert>;
    }

    let selectOne = null;
    let content = null;

    if (this.state.data) {
      if (this.state.data.devices.every(d => !d.is_current)) {
        selectOne = (
          <Alert variant="danger">
            <b>No audio device selected</b><br />
            Press <FontAwesomeIcon icon="play" /> below to select one.
          </Alert>
        );
      }

      if (this.state.data.devices.length === 0) {
        content = (
          <Alert variant="warning">
            No audio devices found, you might have to Authorize Spotify.
            Otherwise try starting a device and refreshing.
          </Alert>
        );
      } else {
        content = (
          <Table responsive="sm">
            <thead>
              <tr>
                <th colSpan="2">Device Name</th>
                <th>Device Type</th>
              </tr>
            </thead>
            <tbody>
              {this.state.data.devices.map((d, id) => {
                if (d.is_current) {
                  return (
                    <tr key={id}>
                      <td width="24" title="Current device">
                        <FontAwesomeIcon icon="volume-up" />
                      </td>
                      <td>
                        {d.name}
                      </td>
                      <td>{d.type}</td>
                    </tr>
                  );
                }

                return (
                  <tr key={id}>
                    <td width="24" title="Switch to device">
                      <FontAwesomeIcon
                        icon="play"
                        className="clickable"
                        onClick={() => this.pickDevice(d.id)} />
                    </td>
                    <td>{d.name}</td>
                    <td>{d.type}</td>
                  </tr>
                );
              })}
            </tbody>
          </Table>
        );
      }
    }

    return (
      <>
        <Loading isLoading={this.state.loading} />
        {error}
        {selectOne}
        {content}
      </>
    );
  }
}