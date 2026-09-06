export class PerceptionError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "PerceptionError";
    this.code = code;
  }
}
