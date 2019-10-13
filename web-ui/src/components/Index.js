import React from "react";
import { Card, CardDeck, Row, Col } from "react-bootstrap";
import { RouteLayout } from "./Layout.js";
import twitchDarkLogo from "../assets/twitch-dark.png";
import windowsImg from "../assets/windows.svg";
import debianImg from "../assets/debian.svg";
import macImg from "../assets/mac.svg";
import SVG from 'react-inlinesvg';
import { api } from '../globals.js';
import Loading from 'shared-ui/components/Loading';
import logo from "../assets/logo.png";

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
  let stable = latestReleases(releases.filter(r => !r.prerelease), 2);
  let unstable = latestReleases(releases.filter(r => r.prerelease), 2);
  return {stable, unstable};
}

/**
 * Get the latest release out of a collection of releases.
 *
 * @param {*} releases
 */
function latestReleases(releasesIn, n) {
  let releases = [];

  for (let release of releasesIn) {
    try {
      releases.push({version: new Version(release.tag_name), release});
    } catch(e) {
      continue;
    }
  }

  releases.sort((a, b) => b.version.cmp(a.version));
  return releases.slice(0, Math.min(n, releases.length));
}

function partitionDownloads(incoming, unstable) {
  return incoming.map(({release, version}) => {
    let debian = [];
    let windows = [];
    let mac = [];

    for (let asset of release.assets) {
      if (asset.name.endsWith(".deb")) {
        debian.push({asset, title: `Package`, prio: 1});
        continue;
      }

      if (asset.name.endsWith(".msi")) {
        windows.push({asset, title: `Installer`, prio: 1});
        continue;
      }

      if (asset.name.endsWith(".zip")) {
        if (asset.name.indexOf("windows") != -1) {
          windows.push({asset, title: `Zip Archive`, prio: 0});
        } else if (asset.name.indexOf("linux") != -1) {
          debian.push({asset, title: `Zip Archive`, prio: 0});
        } else if (asset.name.indexOf("macos") != -1) {
          mac.push({asset, title: `Zip Archive`, prio: 0});
        }

        continue;
      }
    }

    return {version, unstable, debian, windows, mac};
  });
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
    stable = partitionDownloads(stable, false);
    unstable = partitionDownloads(unstable, true);
    this.setState({ releases, stable, unstable, loadingReleases: false });
  }

  async componentDidMount() {
    await this.refreshReleases();
  }

  /**
   * Optionally render download links in case they are available.
   */
  renderDownloadLinks(data, filter) {
    return data.flatMap(({version, unstable, ...other}) => {
      return (filter(other) || []).map(({asset, title, prio}) => {
        let m = asset.name.match(/\.[a-z]+$/);

        let ext = null;

        if (m !== null) {
          ext = <> ({m[0]})</>;
        }

        let unstableEl = null;

        if (unstable) {
          unstableEl = <> <span className="oxi-unstable" title="Development version with new features, but has a higher risk of bugs">DEV</span></>;
        }

        let element = (key) => <Card.Text key={key}>
          <a href={asset.browser_download_url}><b>{version.toString()}</b> &ndash; {title}{ext}</a>{unstableEl}
        </Card.Text>;

        return {element, version, prio};
      });
    });
  }

  renderCard(filter, title, img) {
    let releases = [];

    if (!this.state.loadingReleases) {
      releases.push(...this.renderDownloadLinks(this.state.stable, filter));
      releases.push(...this.renderDownloadLinks(this.state.unstable, filter));

      releases.sort((a, b) => {
        let byVersion = b.version.cmp(a.version);

        if (byVersion === 0) {
          return b.prio - a.prio;
        }

        return byVersion;
      });
    }

    if (releases.length > 0) {
      releases = releases.map((r, i) => r.element(i));
    } else {
      releases = <Card.Text key="no-release" className="oxi-center">No Releases Yet!</Card.Text>;
    }

    return <Card>
      <Card.Img as={SVG} src={img} height="80px" className="mb-3 mt-3" />
      <Card.Body>
        <Card.Title className="oxi-center">{title}</Card.Title>
        <Loading isLoading={this.state.loadingReleases} />
        {releases}
      </Card.Body>
    </Card>;
  }

  render() {
    let windowsCard = this.renderCard(r => r.windows, "Windows", windowsImg);
    let debianCard = this.renderCard(r => r.debian, "Debian", debianImg);
    let macCard = this.renderCard(r => r.mac, "Mac OS", macImg);

    return (
      <RouteLayout>
        <Row className="oxi-intro">
          <Col sm="8">
            <h1 className="oxi-title">OxidizeBot</h1>

            <p>
              The high octane <a href="https://twitch.tv"><img src={twitchDarkLogo} height="16px" width="48px" alt="twitch" /></a> bot.
            </p>

            <p>
              <b>OxidizeBot</b> as an open source Twitch Bot empowering you to focus on what's important.
            </p>

            <p>
              It allows for a richer interaction between you and your chat.
              From a song request system, to groundbreaking game modes where your viewers can interact directly with you and your game.
            </p>

            <p>
              It's written in <a href="https://rust-lang.org">Rust</a>, providing an unparalleled level of reliability and performance.
            </p>
          </Col>

          <Col sm="4" className="oxi-logo-big">
            <img src={logo} />
          </Col>
        </Row>

        <CardDeck className="mb-4">
          <Card>
            <Card.Body>
              <Card.Title className="oxi-center"><b>Free</b> and <b>Open Source</b></Card.Title>
              <Card.Text>
                OxidizeBot doesn't cost you anything,
                and its source code is available on <a href="https://github.com/udoprog/OxidizeBot">GitHub</a> for anyone to tinker with!
              </Card.Text>
            </Card.Body>
          </Card>

          <Card>
            <Card.Body>
              <Card.Title className="oxi-center"><b>Packed</b> with <b>Features</b></Card.Title>
              <Card.Text>
                Plays music, moderates your chat, plays games, you name it!
              </Card.Text>
              <Card.Text>
                If you feel something is missing, feel free to <a href="https://github.com/udoprog/OxidizeBot/issues">open an issue</a>.
              </Card.Text>
            </Card.Body>
          </Card>

          <Card>
            <Card.Body>
              <Card.Title className="oxi-center">Runs on <b>Your Computer</b></Card.Title>
              <Card.Text>
                <em>You</em> own your data.
                It uses <em>your</em> internet for the best possible latency.
                It's light on system resources*.
                And running locally means it can perform rich interactions with your games like <a href="https://github.com/udoprog/ChaosMod">Chaos%</a>.
              </Card.Text>

              <div className="oxi-subtext">
                *: Low CPU usage and about 50MB of ram.
              </div>
            </Card.Body>
          </Card>
        </CardDeck>

        <h4 className="oxi-center mb-4">Downloads</h4>

        <CardDeck>
          {windowsCard}
          {debianCard}
          {macCard}
        </CardDeck>
      </RouteLayout>
    );
  }
}