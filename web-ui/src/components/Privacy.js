import React from "react";
import { Link } from "react-router-dom";
import { RouteLayout } from "./Layout.js";

export default class Privacy extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    return (
      <RouteLayout>
        <h1 className="oxi-page-title">Privacy Policy</h1>

        <p>Effective date: October 4, 2019</p>

        <p>
          setbac.tv ("us", "we", or "our") operates the OxidizeBot Desktop application and Service  (the "Service").
        </p>

        <p>
          This page informs you of our policies regarding the collection, use, and disclosure of personal data when you use our Service and the choices you have associated with that data.
        </p>

        <p>
          <b>We don't collect any personal data about our users.</b> This service will only ever store OAuth 2.0 access tokens which are made available to the OxidizeBot desktop application at your request.
        </p>

        <p>
          At any time, you can revoke this consent under <Link to="/connections">My Connections</Link>. After which <em>all data</em> associated with the connection will be deleted.
        </p>

        <h2>
          Changes To This Privacy Policy
        </h2>

        <p>
          We may update our Privacy Policy from time to time. We will notify you of any changes by posting the new Privacy Policy on this page.
        </p>
        <p>
          We will let you know via email and/or a prominent notice on our Service, prior to the change becoming effective and update the "effective date" at the top of this Privacy Policy.
        </p>
        <p>
          You are advised to review this Privacy Policy periodically for any changes. Changes to this Privacy Policy are effective when they are posted on this page.
        </p>

        <h2>Contact Us</h2>

        <p>
          If you have any questions about this Privacy Policy, please contact us:
        </p>

        <ul>
          <li>By email: <a href="mailto:udoprog@tedro.se">udoprog@tedro.se</a></li>
        </ul>
      </RouteLayout>
    );
  }
}