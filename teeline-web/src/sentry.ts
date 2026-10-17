import * as Sentry from '@sentry/browser'

Sentry.init({
  dsn: 'https://ec76cc3371026e1e4feb8ececea14aee@o4511523471228928.ingest.de.sentry.io/4511523482107984',
  // `sendDefaultPii` was removed in @sentry/browser v11 in favour of the
  // granular `dataCollection` option. Its v11 default already matches the
  // previous `sendDefaultPii: true` behaviour (including automatic IP address
  // collection on events), so no explicit opt-in is needed.
  tunnel: '/tunnel',
})
