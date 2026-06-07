import * as Sentry from "@sentry/browser";

Sentry.init({
  dsn: "https://ec76cc3371026e1e4feb8ececea14aee@o4511523471228928.ingest.de.sentry.io/4511523482107984",
  // Setting this option to true will send default PII data to Sentry.
  // For example, automatic IP address collection on events
  sendDefaultPii: true,
  tunnel: "/tunnel",
});
