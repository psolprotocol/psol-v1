# =============================================================================
# pSol Relayer - Production Dockerfile
# =============================================================================
# Multi-stage build for minimal production image

# -----------------------------------------------------------------------------
# Stage 1: Builder
# -----------------------------------------------------------------------------
FROM node:20-alpine AS builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache python3 make g++

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci

# Copy source
COPY tsconfig.json ./
COPY src ./src

# Build
RUN npm run build

# Prune dev dependencies
RUN npm prune --production

# -----------------------------------------------------------------------------
# Stage 2: Production
# -----------------------------------------------------------------------------
FROM node:20-alpine AS production

# Security: Run as non-root user
RUN addgroup -g 1001 -S relayer && \
    adduser -S -D -H -u 1001 -G relayer relayer

WORKDIR /app

# Install runtime dependencies only
RUN apk add --no-cache \
    dumb-init \
    curl

# Copy built files from builder
COPY --from=builder --chown=relayer:relayer /app/dist ./dist
COPY --from=builder --chown=relayer:relayer /app/node_modules ./node_modules
COPY --from=builder --chown=relayer:relayer /app/package.json ./

# Create keys directory
RUN mkdir -p /app/keys && chown relayer:relayer /app/keys

# Switch to non-root user
USER relayer

# Environment
ENV NODE_ENV=production
ENV PORT=3000

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Use dumb-init for proper signal handling
ENTRYPOINT ["/usr/bin/dumb-init", "--"]

# Start relayer
CMD ["node", "dist/index.js"]
