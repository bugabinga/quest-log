# Quest Log Implementation Plan

## Project Overview

**Quest Log** is a gamified TODO-list web application for children. Parents set
daily tasks (quests) that kids can complete to earn experience points (EXP). The
app includes weekly goals and rewards system.

### Architecture Decisions

- **Backend**: Rust with Axum web framework
- **Database**: SQLite with SQLx (compile-time checked queries)
- **Frontend**: Server-side rendered HTML with Datastar for reactivity
- **Templating**: Askama (compile-time HTML validation)
- **Error Handling**: Thiserror with HTTP status code mapping
- **Configuration**: Environment variables only
- **Testing**: Unit tests for business logic + integration tests for HTTP
  endpoints

### Tech Stack

```
axum = "0.8"                          # Web framework
datastar = { version = "0.3", features = ["axum"] }  # Reactive UI
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "chrono"] }
askama = "0.12"                       # HTML templating
thiserror = "2.0"                     # Error handling
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
tower-http = { version = "0.6", features = ["fs"] }
dotenvy = "0.15"                      # Environment variables
chrono = { version = "0.4", features = ["serde"] }
```

### Database Schema

#### Core Tables

- **quests**: Tasks with weekday assignments, EXP values, optional BLOB images
- **quest_completions**: Completion tracking by date
- **rewards**: Prizes requiring weekly EXP thresholds, optional BLOB images
- **reward_claims**: Reward claiming history
- **settings**: App configuration (weekly goals, single row table)

#### Key Constraints

- Quests assigned to specific weekdays (0-6)
- Unique completions per quest per day
- Unique reward claims per reward per day
- Images stored as BLOBs with content-type metadata

## Implementation Phases

### Phase 1: Project Setup & Core Infrastructure ✅ COMPLETE

**Status**: All steps completed and verified

#### Step 1.1: Add Dependencies to Cargo.toml ✅

- Added all required dependencies
- Verified compilation with `cargo check` (now part of `just check`)

#### Step 1.2: Create Database Migration File ✅

- Created `migrations/001_initial_schema.sql`
- Verified SQL executes without errors
- Schema includes all tables, indexes, constraints

#### Step 1.3: Set Up Basic Axum Server ✅

- Created `src/main.rs` with basic server
- Environment variable support (PORT, DATABASE_URL)
- Verified server starts and responds to requests

### Phase 2: Database Layer & Models ✅ COMPLETE

**Status**: All steps completed and verified

#### Step 2.1: Create Database Models ✅

- Created `src/models.rs` with all struct definitions
- Added `#[derive(sqlx::FromRow)]` for SQLx compatibility
- Included request/response DTOs

#### Step 2.2: Implement Database Connection ✅

- Created `src/database.rs` with connection pool
- Added migration execution logic
- Comprehensive CRUD operations for all entities

#### Step 2.3: Add Database Migration Logic ✅

- Integrated automatic migrations into startup process
- Verified database creation and schema setup on first run
- Confirmed default settings insertion

**Verification**: Database file created automatically on first application run,
all tables present, indexes created, default data inserted

### Phase 3: Core Business Logic ✅ COMPLETE

**Status**: All steps completed and verified

#### Step 3.1: Implement Quest Operations ✅

- Complete CRUD operations for quests (create, read, update, delete)
- Day-based quest filtering
- Active/inactive quest management

#### Step 3.2: Implement Completion Tracking ✅

- Quest completion toggling with EXP calculation
- Weekly EXP aggregation
- Completion status checking per date

#### Step 3.3: Implement Reward System ✅

- Reward CRUD operations
- Weekly EXP-based eligibility checking
- Reward claiming with duplicate prevention
- Claim history tracking

#### Step 3.4: Comprehensive Unit Testing ✅

- 10 unit tests covering all database functions
- In-memory SQLite databases for isolated testing
- Edge case testing and business rule validation
- All tests passing with `just test-business-logic`

**Verification**: All database operations tested and working, business logic
validated

### Phase 4: HTTP Handlers & Templates

**Status**: Phase 4.1 Complete, Phase 4.2 In Progress

#### Step 4.1: Set Up Askama Templates ✅

- ✅ Create templates directory structure
- ✅ Set up basic template rendering
- ✅ Added askama_axum dependency and dotenvy
- ✅ Created QuestDisplay and QuestsTemplate structs with public fields
- ✅ Implemented template rendering in quests handler
- ✅ Fixed template compilation issues (conditional syntax)
- ✅ Verified template renders HTML correctly with quest data

#### Step 4.2: Implement Main Quest View (/)

- Create quests handler and template
- Display daily quests with completion status

#### Step 4.3: Implement Parent Settings View (/parent)

- Create parent handler for CRUD operations
- Build settings management UI

#### Step 4.4: Implement Highscore View (/highscore)

- Create stats handler and template
- Aggregate historical data

### Phase 5: Datastar Integration & Interactivity

**Status**: Pending

#### Step 5.1: Add Datastar to Base Template

- Include datastar script
- Set up reactive signal system

#### Step 5.2: Implement Quest Toggling

- Add POST /quests/toggle endpoint
- Real-time UI updates with SSE

#### Step 5.3: Implement Settings Updates

- Add form submission handlers
- Live updates for configuration changes

### Phase 6: Styling & Polish

**Status**: Pending

#### Step 6.1: Add CSS Framework

- Implement responsive design
- Child-friendly UI styling

#### Step 6.2: Add Image Upload Support

- Implement image upload endpoints
- Display images in templates

### Phase 7: Testing & Quality Assurance ✅ COMPLETE

**Status**: All steps completed and verified

#### Step 7.1: Integration Test Suite ✅

- Created comprehensive integration tests covering all application layers
- `tests/database_integration.rs`: Database operations and schema validation
- `tests/business_logic_integration.rs`: Complex business logic workflows (EXP
  calculations, reward claiming)
- `tests/web_integration.rs`: Web layer operations and handler logic
  verification
- All tests pass with in-memory SQLite databases for isolation

#### Step 7.2: Streamlined Test Commands ✅

- Updated `justfile` with clean command structure:
  - `just test`: Unit tests only (fast feedback)
  - `just verify`: Integration tests only (comprehensive validation)
- Removed obsolete verify-phase commands and simplified workflow
- Updated AGENTS.md with detailed testing strategy documentation

**Verification**: Full test suite running successfully, clear separation between
unit and integration tests, comprehensive coverage of business logic and data
integrity.

## Current Project State

### Files Created

- `Cargo.toml` - Dependencies configured
- `src/main.rs` - Basic axum server with DB integration
- `src/lib.rs` - Module declarations
- `src/models.rs` - Data structures and DTOs
- `src/database.rs` - Database operations and connection
- `migrations/001_initial_schema.sql` - Complete database schema
- `justfile` - Development commands
- `quest.db` - SQLite database (initialized automatically by application)
- `tests/database_integration.rs` - Database layer integration tests
- `tests/business_logic_integration.rs` - Business logic integration tests
- `tests/web_integration.rs` - Web layer integration tests
- `AGENTS.md` - Agent instructions and testing strategy documentation

### Verified Functionality

- ✅ Server starts successfully
- ✅ Database connects and runs migrations
- ✅ All tables created with proper constraints
- ✅ Default settings initialized
- ✅ Environment variables work
- ✅ Basic HTTP endpoint responds
- ✅ Complete database CRUD operations
- ✅ Quest completion tracking with EXP calculation
- ✅ Reward system with weekly claiming logic
- ✅ Comprehensive unit test suite (10 tests passing)
- ✅ Database integration tests (schema validation, CRUD operations)
- ✅ Business logic integration tests (EXP calculations, reward workflows)
- ✅ Web layer integration tests (handler logic verification)
- ✅ Streamlined testing commands with clear separation

### Next Steps

1. ✅ Phase 4.1 Complete: Askama templates working and rendering quest data
2. Start Phase 4.2: Implement Datastar integration for quest completion
3. Add POST endpoint for toggling quest completion
4. Style the quest interface with CSS

## Development Commands

```bash
# Testing
just test               # Run unit tests only (fast feedback)
just verify             # Run integration tests (comprehensive validation)
just check              # Quality check (lint + unit tests + compile)

# Development
cargo run               # Start development server (auto-initializes database)
cargo check            # Verify compilation

# Quality checks
just lint              # Run linter and formatter
```

## Key Design Patterns

### Error Handling

- Custom `AppError` enum with `thiserror`
- HTTP status code mapping via `axum::response::IntoResponse`
- Database errors converted to appropriate HTTP responses

### Database Operations

- Compile-time SQL verification with SQLx
- Connection pooling with `SqlitePool`
- Transaction support for multi-step operations
- Proper indexing for query performance

### HTTP Architecture

- RESTful endpoints with clear separation
- JSON request/response bodies
- Server-sent events for real-time updates
- State management via database (not in-memory)

### Testing Strategy

- Unit tests for pure business logic functions
- Integration tests for HTTP endpoints
- Database state verification
- Manual testing for UI interactions

## Future Considerations

### Scalability

- Current design supports single-user mode
- Database can be migrated to PostgreSQL if needed
- Connection pooling ready for multi-user scenarios

### Features

- Image upload/download functionality
- User authentication (if expanding beyond single-user)
- Mobile-responsive design
- Offline support via service workers

### Performance

- Database queries optimized with indexes
- Minimal allocations in hot paths
- Efficient template rendering with Askama

---

**Last Updated**: February 9, 2026 **Current Phase**: 4.2 (Datastar Integration)
**Next Milestone**: Quest completion toggling with reactive UI</content>
<parameter name="filePath">PLAN.md
