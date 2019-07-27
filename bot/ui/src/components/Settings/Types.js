import {String} from "./String";
import {Text} from "./Text";
import {Duration} from "./Duration";
import {Boolean} from "./Boolean";
import {Number} from "./Number";
import {Percentage} from "./Percentage";
import {Raw} from "./Raw";
import {Set} from "./Set";
import {Select} from "./Select";
import {Typeahead} from "./Typeahead";
import {Oauth2Config} from "./Oauth2Config";
import {Object} from "./Object";
import * as format from "./Format";
import * as moment from "moment-timezone";

/**
 * Decode the given type and value.
 *
 * @param {object} type the type to decode
 * @param {any} value the value to decode
 */
export function decode(type, what = "thing") {
  if (type === null) {
    throw new Error(`bad type: ${type}`);
  }

  let value = null;

  switch (type.id) {
    case "oauth2-config":
      return new Oauth2Config(type.optional);
    case "duration":
      return new Duration(type.optional);
    case "bool":
      return new Boolean(type.optional);
    case "string":
      return new String(type.optional, format.decode(type.format), type.placeholder);
    case "text":
      return new Text(type.optional);
    case "number":
      return new Number(type.optional);
    case "percentage":
      return new Percentage(type.optional);
    case "set":
      value = decode(type.value);
      return new Set(type.optional, value);
    case "select":
      value = decode(type.value);

      switch (type.variant) {
        case "typeahead":
          return new Typeahead(type.optional, value, type.options, what);
        default:
          return new Select(type.optional, value, type.options);
      }
    case "object":
      let fields = type.fields.map(f => {
        let control = decode(f.type, what = f.title);
        return {control, ...f};
      });

      return new Object(type.optional, fields);
    case "time-zone":
      value = new String(false, new format.None(), "");;
      return new Typeahead(type.optional, value, timezoneOptions, "timezone");
    default:
      return new Raw(type.optional);
  }
}

const timezoneOptions = buildTimezoneOptions();

function buildTimezoneOptions() {
  let out = [];

  for (let name of moment.tz.names()) {
    let now = moment();
    let zone = moment.tz.zone(name);

    let offset = zone.utcOffset(now);
    let abbr = zone.abbr(now);

    let id = null;

    if (offset >= 0) {
      id = `UTC-${tzOffset(offset)}`;
    } else {
      id = `UTC+${tzOffset(-offset)}`;
    }

    name = name.split("/").map(n => n.replace('_', ' ')).join(' / ');

    out.push({
      title: `${id} - ${name} (${abbr})`,
      value: zone.name,
    });
  }

  return out;
}

function tzOffset(offset) {
  let rest = offset % 60;
  return `${pad((offset - rest) / 60, 2)}${pad(rest, 2)}`;

  function pad(num, size) {
    var s = num + "";

    while (s.length < size) {
      s = "0" + s;
    }

    return s;
  }
}