import React from "react";

export default function Loading(props) {
  if (props.isLoading !== undefined && !props.isLoading) {
    return null;
  }

  let info = null;

  if (props.children) {
    info = <div className="oxi-loading-info">{props.children}</div>;
  }

  return (
    <div className="oxi-loading">
      {info}

      <div className="spinner-border" role="status">
        <span className="sr-only">Loading...</span>
      </div>
    </div>
  );
}