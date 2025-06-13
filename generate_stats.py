#!/usr/bin/env python3
"""
Generate detailed statistics and a musical representation of git history
Each commit becomes a note in the Para Symphony
"""

import subprocess
import json
from datetime import datetime
from collections import defaultdict, Counter
import re

def run_git_command(cmd):
    """Execute a git command and return the output"""
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    return result.stdout.strip()

def analyze_git_history():
    """Extract comprehensive statistics from git history"""
    
    # Get all commits with detailed information
    log_format = '%H|%an|%ae|%ad|%s|%b'
    commits_raw = run_git_command(f"git log --pretty=format:'{log_format}' --date=iso-strict --all")
    
    commits = []
    stats = {
        'total_commits': 0,
        'authors': defaultdict(int),
        'commits_by_day': defaultdict(int),
        'commits_by_hour': defaultdict(int),
        'commit_types': defaultdict(int),
        'file_changes': defaultdict(int),
        'languages': defaultdict(int),
        'release_history': [],
        'productivity_score': defaultdict(float)
    }
    
    for line in commits_raw.split('\n'):
        if not line:
            continue
            
        parts = line.split('|')
        if len(parts) >= 5:
            commit_hash = parts[0]
            author = parts[1]
            email = parts[2]
            date_str = parts[3]
            message = parts[4]
            
            # Parse date
            try:
                date = datetime.fromisoformat(date_str.replace('Z', '+00:00'))
            except:
                continue
            
            commit = {
                'hash': commit_hash,
                'author': author,
                'email': email,
                'date': date,
                'message': message,
                'day': date.strftime('%Y-%m-%d'),
                'hour': date.hour,
                'weekday': date.strftime('%A')
            }
            
            commits.append(commit)
            
            # Update statistics
            stats['total_commits'] += 1
            stats['authors'][author] += 1
            stats['commits_by_day'][commit['day']] += 1
            stats['commits_by_hour'][commit['hour']] += 1
            
            # Classify commit type
            msg_lower = message.lower()
            if 'release' in msg_lower or 'version' in msg_lower:
                stats['commit_types']['release'] += 1
                stats['release_history'].append({
                    'date': commit['day'],
                    'message': message
                })
            elif 'fix' in msg_lower:
                stats['commit_types']['fix'] += 1
            elif 'add' in msg_lower or 'implement' in msg_lower:
                stats['commit_types']['feature'] += 1
            elif 'merge' in msg_lower:
                stats['commit_types']['merge'] += 1
            elif 'test' in msg_lower:
                stats['commit_types']['test'] += 1
            else:
                stats['commit_types']['other'] += 1
            
            # Get file changes for this commit
            files_changed = run_git_command(f"git diff-tree --no-commit-id --name-only -r {commit_hash}")
            for file in files_changed.split('\n'):
                if file:
                    stats['file_changes'][file] += 1
                    # Track language by extension
                    if '.' in file:
                        ext = file.split('.')[-1]
                        stats['languages'][ext] += 1
    
    # Calculate productivity score (commits per day with decay for age)
    today = datetime.now()
    for day, count in stats['commits_by_day'].items():
        day_date = datetime.fromisoformat(day)
        age_days = (today - day_date).days
        # Recent commits are weighted more heavily
        weight = 1.0 / (1 + age_days / 30)  # 30-day half-life
        stats['productivity_score'][day] = count * weight
    
    return commits, stats

def create_musical_representation(commits, stats):
    """Convert commit patterns into a musical notation"""
    
    # Map commit types to musical notes (pentatonic scale for harmony)
    note_map = {
        'release': 'C5',  # High C for releases
        'feature': 'A4',  # A for additions
        'fix': 'F4',      # F for fixes
        'merge': 'D4',    # D for merges
        'test': 'G4',     # G for tests
        'other': 'C4'     # Middle C for others
    }
    
    # Map authors to instruments
    authors = list(stats['authors'].keys())
    instruments = ['piano', 'violin', 'cello', 'flute', 'guitar']
    author_instruments = {
        author: instruments[i % len(instruments)] 
        for i, author in enumerate(authors)
    }
    
    # Create a simple musical score
    musical_data = {
        'title': 'The Para Symphony - A Git History in Music',
        'tempo': 120,  # BPM
        'time_signature': '4/4',
        'movements': []
    }
    
    # Group commits by week to create movements
    commits_by_week = defaultdict(list)
    for commit in commits:
        week = commit['date'].strftime('%Y-W%U')
        commits_by_week[week].append(commit)
    
    # Create movements for the most active weeks
    for week, week_commits in sorted(commits_by_week.items())[-10:]:
        movement = {
            'name': f'Week of {week}',
            'notes': []
        }
        
        for commit in week_commits:
            msg_lower = commit['message'].lower()
            commit_type = 'other'
            
            if 'release' in msg_lower or 'version' in msg_lower:
                commit_type = 'release'
            elif 'fix' in msg_lower:
                commit_type = 'fix'
            elif 'add' in msg_lower or 'implement' in msg_lower:
                commit_type = 'feature'
            elif 'merge' in msg_lower:
                commit_type = 'merge'
            elif 'test' in msg_lower:
                commit_type = 'test'
            
            note = {
                'pitch': note_map[commit_type],
                'duration': 1 if commit_type == 'release' else 0.5,  # Releases are longer notes
                'instrument': author_instruments.get(commit['author'], 'piano'),
                'velocity': 100 if commit_type == 'release' else 80,  # Releases are louder
                'time': commit['hour'] / 24.0  # Position in the day
            }
            
            movement['notes'].append(note)
        
        musical_data['movements'].append(movement)
    
    return musical_data

def generate_summary_report(stats):
    """Create a beautiful summary report"""
    
    report = []
    report.append("=" * 60)
    report.append("PARA GIT HISTORY: A COSMIC ANALYSIS")
    report.append("=" * 60)
    report.append("")
    
    # Overall statistics
    report.append(f"Total Commits: {stats['total_commits']}")
    report.append(f"Active Days: {len(stats['commits_by_day'])}")
    report.append(f"Contributors: {len(stats['authors'])}")
    report.append("")
    
    # Top contributors
    report.append("TOP STAR GAZERS:")
    for author, count in sorted(stats['authors'].items(), key=lambda x: x[1], reverse=True)[:5]:
        stars = '⭐' * min(count // 10 + 1, 5)
        report.append(f"  {author}: {count} commits {stars}")
    report.append("")
    
    # Commit type distribution
    report.append("COMMIT CONSTELLATION TYPES:")
    total_typed = sum(stats['commit_types'].values())
    for ctype, count in sorted(stats['commit_types'].items(), key=lambda x: x[1], reverse=True):
        percentage = (count / total_typed) * 100
        bar = '█' * int(percentage / 2)
        report.append(f"  {ctype.capitalize()}: {bar} {percentage:.1f}%")
    report.append("")
    
    # Most productive hours
    report.append("CODING HOURS (24h UTC):")
    hour_chart = [''] * 24
    max_commits = max(stats['commits_by_hour'].values()) if stats['commits_by_hour'] else 1
    for hour in range(24):
        count = stats['commits_by_hour'].get(hour, 0)
        height = int((count / max_commits) * 8)
        hour_chart[hour] = '█' * height if height > 0 else '·'
    
    report.append("  " + ''.join(f"{h:02d} " for h in range(0, 24, 3)))
    report.append("  " + '   '.join(hour_chart[::3]))
    report.append("")
    
    # Language distribution
    report.append("LANGUAGE UNIVERSE:")
    top_langs = sorted(stats['languages'].items(), key=lambda x: x[1], reverse=True)[:10]
    for lang, count in top_langs:
        report.append(f"  .{lang}: {count} files touched")
    report.append("")
    
    # Recent releases
    report.append("RECENT STELLAR EVENTS (Releases):")
    for release in stats['release_history'][-5:]:
        report.append(f"  {release['date']}: {release['message']}")
    report.append("")
    
    # Fun facts
    report.append("COSMIC FACTS:")
    
    # Most productive day
    if stats['commits_by_day']:
        best_day = max(stats['commits_by_day'].items(), key=lambda x: x[1])
        report.append(f"  🌟 Brightest Day: {best_day[0]} with {best_day[1]} commits")
    
    # Productivity trends
    recent_days = sorted(stats['commits_by_day'].keys())[-30:]
    recent_commits = sum(stats['commits_by_day'][day] for day in recent_days)
    avg_recent = recent_commits / len(recent_days) if recent_days else 0
    report.append(f"  📈 Recent Activity: {avg_recent:.1f} commits/day (last 30 days)")
    
    # File champion
    if stats['file_changes']:
        most_changed = max(stats['file_changes'].items(), key=lambda x: x[1])
        report.append(f"  🔥 Hottest File: {most_changed[0]} ({most_changed[1]} changes)")
    
    report.append("")
    report.append("=" * 60)
    
    return '\n'.join(report)

def main():
    print("🌌 Analyzing Para's Git Universe...")
    
    # Analyze git history
    commits, stats = analyze_git_history()
    
    # Generate musical representation
    musical_data = create_musical_representation(commits, stats)
    
    # Save musical data
    with open('git_symphony.json', 'w') as f:
        json.dump(musical_data, f, indent=2, default=str)
    
    # Generate and save summary report
    report = generate_summary_report(stats)
    print(report)
    
    with open('cosmic_analysis.txt', 'w') as f:
        f.write(report)
    
    # Save detailed statistics
    stats_json = {
        'generated_at': datetime.now().isoformat(),
        'total_commits': stats['total_commits'],
        'authors': dict(stats['authors']),
        'commit_types': dict(stats['commit_types']),
        'languages': dict(stats['languages']),
        'recent_activity': {
            day: count for day, count in 
            sorted(stats['commits_by_day'].items())[-30:]
        }
    }
    
    with open('git_stats.json', 'w') as f:
        json.dump(stats_json, f, indent=2)
    
    print("\n✨ Analysis complete! Check out:")
    print("  - cosmic_analysis.txt: Detailed statistics report")
    print("  - git_symphony.json: Musical representation of commits")
    print("  - git_stats.json: Raw statistics data")
    print("  - index.html: Interactive constellation visualization")

if __name__ == '__main__':
    main()