FROM node:22-alpine AS build
WORKDIR /app

COPY apps/web/package*.json ./
RUN npm ci

COPY apps/web/ ./
ARG VITE_WORLD_WS_URL=ws://localhost:8080/ws
ARG VITE_WORLD_API_URL=http://localhost:8080
ENV VITE_WORLD_WS_URL=$VITE_WORLD_WS_URL
ENV VITE_WORLD_API_URL=$VITE_WORLD_API_URL
RUN npm run build

FROM nginx:1.27-alpine
COPY apps/web/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 CMD wget -qO- http://127.0.0.1/healthz || exit 1
