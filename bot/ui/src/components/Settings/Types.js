import {String} from "./String";
import {Duration} from "./Duration";
import {Boolean} from "./Boolean";
import {Number} from "./Number";
import {Percentage} from "./Percentage";
import {Raw} from "./Raw";
import {Set} from "./Set";

/**
 * Decode the given type and value.
 *
 * @param {object} type the type to decode
 * @param {any} value the value to decode
 */
export function decode(type) {
  if (type === null) {
    return Raw;
  }

  switch (type.id) {
    case "duration":
      return new Duration(type.optional);
    case "bool":
      return new Boolean(type.optional);
    case "string":
      return new String(type.optional);
    case "number":
      return new Number(type.optional);
    case "percentage":
      return new Percentage(type.optional);
    case "set":
      let value = decode(type.value);
      return new Set(type.optional, value);
    default:
      return new Raw(type.optional);
  }
}