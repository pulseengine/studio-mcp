# Pipeline Automation Examples

Common automation workflows using the WindRiver Studio MCP Server with AI assistants.

## Basic Pipeline Operations

### Listing and Monitoring Pipelines

**Query**: "Show me all my active pipelines"

**Expected Response**: Claude will use the `plm_list_pipelines` tool to show:
```
Found 3 active pipelines:

1. **Build Pipeline** (pipeline-123)
   - Status: running
   - Project: MyProject
   - Started: 2024-01-20 14:22:00

2. **Deploy Pipeline** (pipeline-456) 
   - Status: idle
   - Project: MyProject
   - Last run: 2024-01-20 13:45:00

3. **Test Pipeline** (pipeline-789)
   - Status: completed
   - Project: QAProject
   - Completed: 2024-01-20 14:30:00
```

### Pipeline Status Monitoring

**Query**: "What's the status of pipeline XYZ-123?"

**Response**: Detailed pipeline information including:
- Current status and progress
- Running tasks and their status
- Estimated completion time
- Recent execution history

### Task-Level Monitoring

**Query**: "Show me all failed tasks in pipeline XYZ-123"

**Response**: List of failed tasks with:
- Task names and failure reasons
- Timestamps of failures
- Links to logs and artifacts
- Suggested remediation steps

## Automated Pipeline Management

### Starting Pipelines

**Query**: "Start the deployment pipeline for project MyApp"

**Workflow**:
1. Claude identifies the deployment pipeline for MyApp
2. Checks if any prerequisite pipelines need to run
3. Starts the pipeline with appropriate parameters
4. Provides run ID and monitoring information

**Example Response**:
```
Starting deployment pipeline for MyApp...

✅ Pipeline "MyApp Deploy" (pipeline-456) started successfully
   Run ID: run-789
   Status: running
   
You can monitor progress with: "What's the status of run run-789?"
```

### Conditional Pipeline Execution

**Query**: "If the build pipeline completed successfully, start the deployment pipeline"

**Workflow**:
1. Check status of build pipeline
2. Verify completion status and success
3. If successful, start deployment pipeline
4. If failed, report issues and suggest fixes

### Batch Operations

**Query**: "Stop all running pipelines in project TestProject"

**Workflow**:
1. List all pipelines in TestProject
2. Filter for running status
3. Stop each running pipeline
4. Report results and any errors

## Advanced Automation Scenarios

### Release Automation

**Query**: "Prepare a release for version 1.2.3 - run build, test, and deploy pipelines in sequence"

**Workflow**:
1. **Build Phase**:
   - Start build pipeline with version 1.2.3 parameter
   - Monitor build progress
   - Wait for completion

2. **Test Phase**:
   - If build successful, start test pipeline
   - Monitor test execution
   - Collect test results

3. **Deploy Phase**:
   - If tests pass, start deployment pipeline
   - Monitor deployment progress
   - Verify deployment success

**Example Implementation**:
```
🚀 Starting release automation for version 1.2.3

Phase 1: Build
├── ✅ Build pipeline started (run-101)
├── ⏳ Waiting for build completion...
└── ✅ Build completed successfully (15:23)

Phase 2: Test
├── ✅ Test pipeline started (run-102)  
├── ⏳ Running unit tests...
├── ⏳ Running integration tests...
└── ✅ All tests passed (15:45)

Phase 3: Deploy
├── ✅ Deployment pipeline started (run-103)
├── ⏳ Deploying to staging...
├── ✅ Staging deployment successful
├── ⏳ Deploying to production...
└── ✅ Production deployment successful (16:12)

🎉 Release 1.2.3 completed successfully!
```

### Failure Recovery

**Query**: "Pipeline XYZ-123 failed - help me diagnose and fix the issue"

**Workflow**:
1. **Analyze Failure**:
   - Get pipeline details and failure information
   - Identify failed tasks and stages
   - Collect error logs and diagnostics

2. **Root Cause Analysis**:
   - Examine error patterns
   - Check dependencies and prerequisites
   - Compare with previous successful runs

3. **Suggest Remediation**:
   - Provide specific fix recommendations
   - Suggest parameter adjustments
   - Recommend resource or configuration changes

4. **Automated Retry**:
   - If simple fix, offer to restart pipeline
   - Monitor retry progress
   - Escalate if retry fails

### Performance Monitoring

**Query**: "Show me a performance summary of all pipelines this week"

**Response**:
```
📊 Pipeline Performance Summary (Week of Jan 15-21, 2024)

Overall Statistics:
├── Total Runs: 156
├── Success Rate: 89.1% (139 successful, 17 failed)
├── Average Duration: 12.5 minutes
└── Total Compute Time: 32.5 hours

Top Performing Pipelines:
1. Unit Test Pipeline - 100% success (45 runs)
2. Build Pipeline - 95.2% success (42 runs)
3. Integration Pipeline - 90.0% success (30 runs)

Problematic Pipelines:
1. E2E Test Pipeline - 70.0% success (20 runs)
   └── Common failure: timeout in UI tests
2. Deploy Pipeline - 75.0% success (16 runs)  
   └── Common failure: network connectivity

Recommendations:
├── Increase timeout for E2E tests from 30m to 45m
├── Add retry logic to deployment network calls
└── Consider splitting large pipelines into smaller stages
```

## Custom Automation Scripts

### Daily Status Report

**Query**: "Generate a daily status report for all my projects"

**Automation**:
```python
# Daily automation script (pseudo-code)
def daily_report():
    projects = get_all_projects()
    report = {}
    
    for project in projects:
        pipelines = get_project_pipelines(project)
        report[project] = {
            'active_pipelines': count_active(pipelines),
            'failed_pipelines': count_failed(pipelines),
            'success_rate': calculate_success_rate(pipelines),
            'recent_issues': get_recent_issues(pipelines)
        }
    
    return format_report(report)
```

### Automated Rollback

**Query**: "If deployment pipeline fails, automatically rollback to previous version"

**Logic**:
1. Monitor deployment pipeline status
2. On failure, identify previous successful version
3. Start rollback pipeline with previous version
4. Verify rollback success
5. Send notifications to team

### Resource Optimization

**Query**: "Identify pipelines that are using too many resources and suggest optimizations"

**Analysis**:
1. Collect pipeline resource usage metrics
2. Compare against baselines and thresholds
3. Identify resource-intensive operations
4. Suggest optimization strategies:
   - Parallel execution opportunities
   - Resource limit adjustments
   - Stage consolidation options
   - Caching improvements

## Integration Patterns

### Slack Integration

Set up Claude Desktop to work with Slack for team notifications:

**Example Workflow**:
1. Pipeline fails in Studio
2. Claude detects failure through MCP server
3. Formats failure summary with logs and context
4. Posts to team Slack channel with @mentions

### Email Reporting

**Query**: "Send me a weekly email summary of all pipeline activity"

**Implementation**:
- Schedule regular queries to Claude
- Generate comprehensive pipeline reports
- Format as professional email summary
- Include charts and metrics

### Dashboard Integration

**Query**: "Update my pipeline dashboard with current status"

**Workflow**:
1. Collect current pipeline status for all projects
2. Format data for dashboard consumption
3. Update dashboard widgets and metrics
4. Trigger refresh of monitoring displays

## Best Practices for Automation

### Error Handling

1. **Always check prerequisites** before starting pipelines
2. **Implement retry logic** for transient failures
3. **Set appropriate timeouts** for long-running operations
4. **Provide detailed error messages** with actionable information

### Monitoring and Alerting

1. **Monitor automation health** - ensure scripts are running
2. **Set up failure alerts** for critical automation workflows
3. **Track automation metrics** - success rates, response times
4. **Review and optimize** automation rules regularly

### Security Considerations

1. **Use service accounts** for automated operations
2. **Limit automation permissions** to minimum required
3. **Audit automation activities** regularly
4. **Secure credential storage** for automated authentication

### Performance Optimization

1. **Use caching effectively** to reduce API calls
2. **Batch operations** when possible
3. **Implement rate limiting** to avoid overloading Studio
4. **Monitor resource usage** of automation scripts