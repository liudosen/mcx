# Security Review Command

## Description
Perform security-focused analysis of code changes.

## When to Use
- Adding authentication/authorization features
- Handling user input
- Processing sensitive data
- Any public API endpoint changes

## Security Checklist

### Authentication
- [ ] JWT secret is strong and from environment
- [ ] Token expiration is reasonable
- [ ] Password hashing uses bcrypt with appropriate cost
- [ ] Invalid credentials don't reveal which field is wrong

### Authorization
- [ ] Endpoints check permissions appropriately
- [ ] Role-based access is enforced
- [ ] Admin-only endpoints are protected

### Input Validation
- [ ] All user input is validated
- [ ] SQL injection prevented via parameterized queries
- [ ] No SQL concatenation with user input

### Data Protection
- [ ] Sensitive data not logged
- [ ] Error messages don't leak internals
- [ ] Database errors handled gracefully

### API Security
- [ ] CORS configured appropriately
- [ ] No hardcoded credentials
- [ ] Environment variables for secrets

## Usage
`/security <description of what to check>`

## In Project Context
For welfare-store:
- Auth endpoints: login, refresh, logout
- JWT validation on protected routes
- Role checks: admin, operator, viewer
