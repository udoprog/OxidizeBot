import {Spinner} from "../utils.js";
import React from "react";
import {Alert, Table} from "react-bootstrap";
import {Play, Speaker} from "react-feather";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";

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

  componentWillMount() {
    if (this.state.loading) {
      return;
    }

    this.listDevices()
  }

  /**
   * Refresh the list of devices.
   */
  listDevices() {
    this.setState({
      loading: true,
    });

    this.api.devices()
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
          error: `failed to request devices: ${e}`,
          data: null,
        });
      });
  }

  /**
   * Pick the specified device.
   *
   * @param {string} id the device to pick.
   */
  pickDevice(id) {
    if (this.state.loading) {
      return;
    }

    this.setState({
      loading: true,
    });

    this.api.setDevice(id)
      .then(_ => {
        return this.listDevices();
      },
      e => {
        this.setState({
          loading: false,
          error: `failed to pick device: ${e}`,
        });
      });
  }

  render() {
    let error = null;

    if (this.state.error) {
      error = <Alert variant="warning">{this.state.error}</Alert>;
    }

    let current = null;

    if (this.state.data && this.state.data.current) {
      current = (
        <div>
          <p>
            Make sure you have the following in your <code>config.toml</code> if you want your current device saved:
          </p>

          <code><pre>{
            `[player]\ndevice = "${this.state.data.current.name}"`
          }</pre></code>
      </div>
      );
    }

    let refresh = null;
    let loading = null;

    if (this.state.loading) {
      loading = <Spinner />;
      refresh = <FontAwesomeIcon icon="sync" className="title-refresh" />;
    } else {
      refresh = <FontAwesomeIcon icon="sync" className="title-refresh clickable" onClick={() => this.listDevices()} />;
    }

    let content = null;

    if (this.state.data) {
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
                        size="24"
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
      <div>
        <h4>
          Devices
          {refresh}
        </h4>
        {error}
        {current}
        {content}
        {loading}
      </div>
    );
  }
}