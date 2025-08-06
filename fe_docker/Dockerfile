FROM node:20-alpine AS builder

# Add build argument for git commit
ARG GIT_COMMIT=unknown
LABEL git.commit=${GIT_COMMIT}

# Install build dependencies
RUN apk add --no-cache python3 make g++ linux-headers eudev-dev

# Set working directory
WORKDIR /app

# Copy package files
COPY target/package.json target/yarn.lock ./

# Install dependencies
RUN yarn --frozen-lockfile install

# Set build-time environment variables
ARG NEXT_PUBLIC_PROJECT_ID
ARG ALCHEMY_API_KEY
ARG SOLVER_TESTNET_URL
ARG SOLVER_MAINNET_URL
ARG NEXT_PUBLIC_FORCE_SOLANA_NETWORK
ENV NEXT_PUBLIC_PROJECT_ID=${NEXT_PUBLIC_PROJECT_ID}
ENV ALCHEMY_API_KEY=${ALCHEMY_API_KEY}
ENV SOLVER_TESTNET_URL=${SOLVER_TESTNET_URL}
ENV SOLVER_MAINNET_URL=${SOLVER_MAINNET_URL}
ENV NEXT_PUBLIC_FORCE_SOLANA_NETWORK=${NEXT_PUBLIC_FORCE_SOLANA_NETWORK}

# Copy source code
COPY target/ ./

# Build the application
RUN yarn build

# Production stage
FROM node:20-alpine AS runner

# Add build argument for git commit
ARG GIT_COMMIT=unknown
LABEL git.commit=${GIT_COMMIT}

WORKDIR /app

# Create non-root user
RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs

# Copy package files and install production dependencies
COPY target/package.json target/yarn.lock ./
RUN yarn --frozen-lockfile install --production --ignore-scripts

# Copy built application from builder stage
COPY --from=builder /app/public ./public
COPY --from=builder /app/.next ./.next

# Set ownership
RUN chown -R nextjs:nodejs /app
USER nextjs

# Add environment variable for runtime access
ENV GIT_COMMIT=${GIT_COMMIT}

# Expose port
EXPOSE 3000

# Start the application
CMD ["yarn", "start"] 