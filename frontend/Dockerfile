# syntax=docker/dockerfile:1

FROM node:22-alpine AS build
WORKDIR /src
ENV VITE_API_BASE_URL=
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
# No VITE_API_BASE_URL is supplied: browser requests stay same-origin.
RUN npm run build

FROM node:22-alpine AS runtime
WORKDIR /app
COPY --from=build /src/dist ./dist
COPY server.mjs ./server.mjs
USER node
ENV PORT=8080 \
    API_UPSTREAM=http://api:3000
EXPOSE 8080
ENTRYPOINT ["node", "server.mjs"]
