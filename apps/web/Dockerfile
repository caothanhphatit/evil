FROM node:22-alpine AS build
WORKDIR /app

COPY apps/web/package*.json ./
RUN npm ci

COPY apps/web/ ./
COPY packages/content/releases/evil-hunter-1.411/building-registry.json /app/public/content/releases/evil-hunter-1.411/building-registry.json
COPY packages/content/releases/evil-hunter-1.411/gear-catalog.json /app/public/content/releases/evil-hunter-1.411/gear-catalog.json
# The full catalog is served at /game-assets/ in production as well as dev.
# Keep it in the image so runtime URLs do not depend on a compose-only mount.
COPY game-assets/extracted/exported/ /app/public/game-assets/
ARG VITE_WORLD_WS_URL
ARG VITE_WORLD_API_URL
ENV VITE_WORLD_WS_URL=$VITE_WORLD_WS_URL
ENV VITE_WORLD_API_URL=$VITE_WORLD_API_URL
RUN npm run build

FROM nginx:1.27-alpine
COPY apps/web/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 CMD wget -qO- http://127.0.0.1/healthz || exit 1
