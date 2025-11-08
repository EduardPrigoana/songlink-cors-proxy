# Songlink CORS Proxy

High-performance CORS proxy for the Songlink API (v1-alpha.1) built in Rust. Provides header and user agent rotation to avoid fingerprinting and rate limiting.

## Features

- Full CORS support with configurable origins
- Automatic header rotation (Accept, Accept-Language, Accept-Encoding, Connection, DNT)
- User agent rotation across 12 different browser profiles
- Automatic gzip/brotli/deflate decompression
- Environment-based origin restrictions
- Production-ready with optimized release builds

## Installation

```bash
git clone <repository-url>
cd songlink-cors-proxy
cargo build --release
```

## Configuration

The proxy supports two modes controlled by the `DEV` environment variable:

**Development Mode** (`DEV=true`)
- Allows requests from `http://localhost:*` and `http://127.0.0.1:*`

**Production Mode** (`DEV=false` or unset)
- Only allows requests from:
  - `https://monochrome.tf`
  - `https://monochrome.prigoana.com`

## Running

```bash
# Development mode
DEV=true cargo run --release

# Production mode
cargo run --release
```

The server runs on `http://0.0.0.0:3000` by default.

## API Endpoints

### `GET /`
Redirects to `https://monochrome.tf`

### `GET /health`
Health check endpoint. Returns `OK`.

### `GET /api/links`
Main proxy endpoint for Songlink API requests.

## Query Parameters

All parameters from the Songlink API v1-alpha.1 are supported:

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `url` | string | Yes* | URL-encoded streaming URL from any supported platform |
| `userCountry` | string | No | Two-letter country code (default: US) |
| `songIfSingle` | boolean | No | Return song data for single-song albums (default: false) |
| `platform` | string | No* | Platform identifier (required if url not provided) |
| `type` | string | No* | Entity type: `song` or `album` (required if url not provided) |
| `id` | string | No* | Platform-specific entity ID (required if url not provided) |
| `key` | string | No | Songlink API key for higher rate limits |

*Either `url` must be provided, or all three of `platform`, `type`, and `id`.

## Supported Platforms

spotify, itunes, appleMusic, youtube, youtubeMusic, google, googleStore, pandora, deezer, tidal, amazonStore, amazonMusic, soundcloud, napster, yandex, spinrilla, audius, anghami, boomplay, audiomack, bandcamp

## Usage Examples

### Using URL parameter

```bash
curl "http://localhost:3000/api/links?url=https%3A%2F%2Fopen.spotify.com%2Ftrack%2F2TmqHjg7uhizGndzXQdFuf&userCountry=US&songIfSingle=true"
```

### Using platform, type, and id parameters

```bash
curl "http://localhost:3000/api/links?platform=spotify&type=song&id=2TmqHjg7uhizGndzXQdFuf&userCountry=US"
```

### From JavaScript

```javascript
const url = encodeURIComponent('https://open.spotify.com/track/2TmqHjg7uhizGndzXQdFuf');
const response = await fetch(
  `http://localhost:3000/api/links?url=${url}&userCountry=US&songIfSingle=true`
);
const data = await response.json();
```

### With API Key

```bash
curl "http://localhost:3000/api/links?url=https%3A%2F%2Fopen.spotify.com%2Ftrack%2F2TmqHjg7uhizGndzXQdFuf&key=YOUR_API_KEY"
```

## Response Format

The proxy returns the same JSON structure as the Songlink API:

```json
{
  "entityUniqueId": "SPOTIFY_SONG::2TmqHjg7uhizGndzXQdFuf",
  "userCountry": "US",
  "pageUrl": "https://song.link/s/2TmqHjg7uhizGndzXQdFuf",
  "linksByPlatform": {
    "spotify": {
      "url": "https://open.spotify.com/track/2TmqHjg7uhizGndzXQdFuf",
      "entityUniqueId": "SPOTIFY_SONG::2TmqHjg7uhizGndzXQdFuf"
    }
  },
  "entitiesByUniqueId": {
    "SPOTIFY_SONG::2TmqHjg7uhizGndzXQdFuf": {
      "id": "2TmqHjg7uhizGndzXQdFuf",
      "type": "song",
      "title": "Be Nice 2 Me",
      "artistName": "Bladee"
    }
  }
}
```

## Error Responses

Errors return JSON with an error message and HTTP status code:

```json
{
  "error": "Failed to fetch from Songlink API: connection timeout",
  "status": 502
}
```

## Rate Limiting

The Songlink API has rate limits:
- Without API key: 10 requests per minute
- With API key: Higher limits (contact Songlink for details)

The proxy does not implement additional rate limiting.

## License

See LICENSE file for details.

## Attribution

This proxy uses the Songlink API. When using this proxy in your application, you should properly attribute that your feature is powered by Songlink as per their API Terms of Service.

## Support

For issues with the Songlink API itself, contact support@odesli.co or developers@song.link
For issues with this proxy, open an issue in the repository.