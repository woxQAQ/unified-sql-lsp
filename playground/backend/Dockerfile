FROM node:20-alpine

WORKDIR /app

# Install backend dependencies
COPY package.json package-lock.json* ./
RUN npm ci --only=production

# Copy backend source
COPY server.js ./

# Expose port
EXPOSE 8080

# Start the server
CMD ["npm", "start"]
