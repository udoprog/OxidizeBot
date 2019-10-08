import React from "react";
import { Card, CardDeck } from "react-bootstrap";
import { RouteLayout } from "./Layout.js";
import dollarImg from "../assets/dollar.png";
import toolboxImg from "../assets/toolbox.png";
import cloudImg from "../assets/cloud.png";
import twitchDarkLogo from "../assets/twitch-dark.png";
import windowsImg from "../assets/windows.svg";
import debianImg from "../assets/debian.svg";
import macImg from "../assets/mac.svg";
import SVG from 'react-inlinesvg';
import { api } from '../globals.js';
import Loading from './Loading';

const VERSION_REGEX = /(\d+)\.(\d+)\.(\d+)(-[a-z]+\.(\d+))?/;

class Version {
  constructor(version) {
    let out = version.match(VERSION_REGEX);

    if (!out) {
      throw new Error("Illegal Version: " + version);
    }

    this.parts = [parseInt(out[1]), parseInt(out[2]), parseInt(out[3]), Infinity];
    let prerelease = out[5];

    if (prerelease !== undefined) {
      this.parts[3] = parseInt(prerelease);
    }

    this.versionString = version;
  }

  cmp(o) {
    for (let i = 0; i < 4; i++) {
      if (this.parts[i] > o.parts[i]) {
        return 1;
      }

      if (this.parts[i] < o.parts[i]) {
        return -1;
      }
    }

    return 0;
  }

  toString() {
    return this.versionString;
  }
}

/**
 * Split releases into a stable and a prerelease.
 *
 * @param {*} releases
 */
function filterReleases(releases) {
  let stable = latestRelease(releases.filter(r => !r.prerelease));
  let unstable = latestRelease(releases.filter(r => r.prerelease));
  return {stable, unstable};
}

/**
 * Get the latest release out of a collection of releases.
 *
 * @param {*} releases
 */
function latestRelease(releases) {
  releases = releases.map(release => {
    return {version: new Version(release.tag_name), release};
  });

  releases.sort((a, b) => b.version.cmp(a.version));

  if (releases.length === 0) {
    return null;
  }

  return releases[0];
}

function partitionDownloads(incoming) {
  let debian = null;
  let windows = null;

  if (incoming === null) {
    return {debian, windows};
  }

  let {release, version} = incoming;

  for (let asset of release.assets) {
    if (asset.name.endsWith(".deb")) {
      debian = {asset, version};
      continue;
    }

    if (asset.name.endsWith(".msi")) {
      windows = {asset, version};
      continue;
    }
  }

  return {debian, windows};
}

export default class Index extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      releases: [],
      stable: null,
      unstable: null,
      loadingReleases: true,
    };
  }

  /**
   * Refresh the known collection of releases.
   */
  async refreshReleases() {
    this.setState({ loadingReleases: true });
    let releases = await api.githubReleases('udoprog', 'OxidizeBot');
    let {stable, unstable} = filterReleases(releases);
    stable = partitionDownloads(stable);
    unstable = partitionDownloads(unstable);
    this.setState({ releases, stable, unstable, loadingReleases: false });
  }

  async componentDidMount() {
    await this.refreshReleases();
  }

  /**
   * Optionally render download links in case they are available.
   *
   * @param {*} data
   * @param {*} filter
   * @param {*} title
   */
  renderDownloadLinks(data, filter, title) {
    if (data === null) {
      return null;
    }

    data = filter(data);

    if (data === null) {
      return null;
    }

    let {asset, version} = data;
    let m = asset.name.match(/\.[a-z]+$/);

    let ext = null;

    if (m !== null) {
      ext = <> ({m[0]})</>;
    }

    return <Card.Text className="center">
      <a href={asset.browser_download_url}>{version.toString()} {title}{ext}</a>
    </Card.Text>;
  }

  renderCard(filter, title, img) {
    let stable = null;
    let unstable = null;

    if (!this.state.loadingReleases) {
      stable = this.renderDownloadLinks(this.state.stable, filter, "Stable Installer");
      unstable = this.renderDownloadLinks(this.state.unstable, filter, "Unstable Installer");
    }

    return <Card bg="light">
      <Card.Img as={SVG} src={img} height="80px" className="mb-3 mt-3" />
      <Card.Body>
        <Card.Title className="center">{title}</Card.Title>
        <Loading isLoading={this.state.loadingReleases} />
        {unstable}
        {stable}
      </Card.Body>
    </Card>;
  }

  render() {
    let windowsCard = this.renderCard(r => r.windows, "Windows", windowsImg);
    let debianCard = this.renderCard(r => r.debian, "Debian", debianImg);

    return (
      <RouteLayout>
        <h2 className="page-title">OxidizeBot</h2>

        <div className="center mb-4">
          <b>OxidizeBot</b> is the high octane <a href="https://twitch.tv"><img src={twitchDarkLogo} height="16px" width="48px" alt="twitch" /></a> bot written in <a href="https://rust-lang.org">Rust</a>!
        </div>

        <CardDeck className="mb-4">
          <Card>
            <Card.Img variant="top" src={dollarImg} />
            <Card.Body>
              <Card.Title className="center"><b>Free</b> and <b>Open Source</b></Card.Title>
              <Card.Text>
                OxidizeBot doesn't cost you anything,
                and its source code is available on <a href="https://github.com/udoprog/OxidizeBot">GitHub</a> for anyone to tinker with!
              </Card.Text>
            </Card.Body>
          </Card>

          <Card>
            <Card.Img variant="top" src={toolboxImg} />
            <Card.Body>
              <Card.Title className="center"><b>Packed</b> with <b>Features</b></Card.Title>
              <Card.Text>
                Plays music, moderates your chat, plays games, you name it!
              </Card.Text>
              <Card.Text>
                If you feel something is missing, feel free to <a href="https://github.com/udoprog/OxidizeBot/issues">open an issue</a>.
              </Card.Text>
            </Card.Body>
          </Card>

          <Card>
            <Card.Img variant="top" src={cloudImg} />
            <Card.Body>
              <Card.Title className="center">Runs on <b>Your Computer</b></Card.Title>
              <Card.Text>
                <em>You</em> own your data.
                It uses <em>your</em> internet for the best possible latency.
                It's light on system resources (Low CPU and about 50MB of ram).
                And running locally means it can perform rich interactions with your games like <a href="https://github.com/udoprog/ChaosMod">Chaos%</a>.
              </Card.Text>
            </Card.Body>
          </Card>
        </CardDeck>

        <h4 className="center mb-4">Download</h4>

        <CardDeck>
          {windowsCard}
          {debianCard}
        </CardDeck>
      </RouteLayout>
    );
  }
}