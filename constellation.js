// Para Git Constellation Visualization
// Transform git history into a celestial map of stars and constellations

class GitConstellation {
    constructor() {
        this.width = window.innerWidth;
        this.height = window.innerHeight;
        this.tooltip = d3.select('.tooltip');
        this.commits = [];
        this.init();
    }

    async init() {
        // Load commit data
        await this.loadData();
        
        // Create SVG canvas
        this.svg = d3.select('#cosmos')
            .append('svg')
            .attr('width', this.width)
            .attr('height', this.height);
        
        // Add gradient definitions
        this.createGradients();
        
        // Create background stars
        this.createBackgroundStars();
        
        // Process and visualize commits
        this.processCommits();
        
        // Update statistics
        this.updateStats();
        
        // Add shooting stars animation
        this.createShootingStars();
        
        // Handle window resize
        window.addEventListener('resize', () => this.handleResize());
    }

    async loadData() {
        try {
            const response = await fetch('commits.json');
            this.commits = await response.json();
        } catch (error) {
            // Fallback to generated data if file doesn't exist
            this.commits = this.generateSampleData();
        }
    }

    generateSampleData() {
        // Generate sample data based on the git history analysis
        const messages = [
            "Bump version to 1.1.23 for release",
            "Fix brew formula to handle existing para-mcp-server symlink",
            "Add MCP commands to CLI autocomplete",
            "Implement MCP integration with para mcp init command",
            "Fix test isolation to prevent IDE termination",
            "Refactor dispatch to support Cursor with template-based AppleScript",
            "Add comprehensive unit tests for para continue",
            "Merge release branch v1.1.22",
            "Update CLAUDE.md with test utilities best practices",
            "Fix flaky test in cancel command"
        ];
        
        const authors = ["2mawi2", "GitHub Action", "Dependabot"];
        const now = Date.now() / 1000;
        
        return Array.from({length: 100}, (_, i) => ({
            hash: Math.random().toString(36).substr(2, 9),
            author: authors[Math.floor(Math.random() * authors.length)],
            date: new Date((now - i * 3600 * 6) * 1000).toISOString(),
            message: messages[Math.floor(Math.random() * messages.length)],
            timestamp: now - i * 3600 * 6
        }));
    }

    createGradients() {
        const defs = this.svg.append('defs');
        
        // Star glow gradient
        const starGlow = defs.append('radialGradient')
            .attr('id', 'star-glow');
        
        starGlow.append('stop')
            .attr('offset', '0%')
            .attr('stop-color', '#ffffff')
            .attr('stop-opacity', 1);
        
        starGlow.append('stop')
            .attr('offset', '50%')
            .attr('stop-color', '#ffffff')
            .attr('stop-opacity', 0.5);
        
        starGlow.append('stop')
            .attr('offset', '100%')
            .attr('stop-color', '#ffffff')
            .attr('stop-opacity', 0);
    }

    createBackgroundStars() {
        // Create a field of small background stars
        const backgroundStars = Array.from({length: 200}, () => ({
            x: Math.random() * this.width,
            y: Math.random() * this.height,
            r: Math.random() * 1.5
        }));
        
        this.svg.append('g')
            .attr('class', 'background-stars')
            .selectAll('circle')
            .data(backgroundStars)
            .enter()
            .append('circle')
            .attr('cx', d => d.x)
            .attr('cy', d => d.y)
            .attr('r', d => d.r)
            .attr('fill', 'white')
            .attr('opacity', d => 0.2 + Math.random() * 0.3);
    }

    processCommits() {
        // Group commits by day to create constellations
        const commitsByDay = {};
        
        this.commits.forEach(commit => {
            const date = new Date(commit.timestamp * 1000).toDateString();
            if (!commitsByDay[date]) {
                commitsByDay[date] = [];
            }
            commitsByDay[date].push(commit);
        });
        
        // Convert to array and sort by date
        const constellations = Object.entries(commitsByDay)
            .map(([date, commits]) => ({date, commits}))
            .sort((a, b) => new Date(b.date) - new Date(a.date));
        
        // Create constellation layout
        this.createConstellations(constellations);
    }

    createConstellations(constellations) {
        const constellationGroup = this.svg.append('g')
            .attr('class', 'constellations');
        
        // Layout constellations in a spiral pattern
        const centerX = this.width / 2;
        const centerY = this.height / 2;
        const spiralRadius = Math.min(this.width, this.height) * 0.35;
        
        constellations.forEach((constellation, i) => {
            const angle = (i / constellations.length) * Math.PI * 4; // 2 full rotations
            const radius = spiralRadius * (1 - i / constellations.length);
            const x = centerX + Math.cos(angle) * radius;
            const y = centerY + Math.sin(angle) * radius;
            
            this.drawConstellation(constellationGroup, constellation, x, y, i);
        });
    }

    drawConstellation(group, constellation, centerX, centerY, index) {
        const stars = constellation.commits;
        const constellationRadius = 30 + stars.length * 5;
        
        // Create constellation group
        const g = group.append('g')
            .attr('class', `constellation-${index}`);
        
        // Draw constellation lines
        if (stars.length > 1) {
            const lineData = [];
            stars.forEach((star, i) => {
                if (i < stars.length - 1) {
                    const angle1 = (i / stars.length) * Math.PI * 2;
                    const angle2 = ((i + 1) / stars.length) * Math.PI * 2;
                    
                    lineData.push({
                        x1: centerX + Math.cos(angle1) * constellationRadius,
                        y1: centerY + Math.sin(angle1) * constellationRadius,
                        x2: centerX + Math.cos(angle2) * constellationRadius,
                        y2: centerY + Math.sin(angle2) * constellationRadius
                    });
                }
            });
            
            g.selectAll('line')
                .data(lineData)
                .enter()
                .append('line')
                .attr('class', 'constellation-line')
                .attr('x1', d => d.x1)
                .attr('y1', d => d.y1)
                .attr('x2', d => d.x2)
                .attr('y2', d => d.y2);
        }
        
        // Draw stars
        stars.forEach((star, i) => {
            const angle = (i / stars.length) * Math.PI * 2;
            const x = centerX + Math.cos(angle) * constellationRadius;
            const y = centerY + Math.sin(angle) * constellationRadius;
            
            const starColor = this.getStarColor(star.message);
            const starSize = this.getStarSize(star.message);
            
            // Star glow effect
            g.append('circle')
                .attr('cx', x)
                .attr('cy', y)
                .attr('r', starSize * 3)
                .attr('fill', starColor)
                .attr('opacity', 0.1);
            
            // Main star
            g.append('circle')
                .attr('class', 'star')
                .attr('cx', x)
                .attr('cy', y)
                .attr('r', starSize)
                .attr('fill', starColor)
                .on('mouseover', (event) => this.showTooltip(event, star))
                .on('mouseout', () => this.hideTooltip())
                .on('click', () => this.onStarClick(star));
            
            // Add twinkle animation
            g.append('circle')
                .attr('cx', x)
                .attr('cy', y)
                .attr('r', starSize)
                .attr('fill', 'white')
                .attr('opacity', 0)
                .append('animate')
                .attr('attributeName', 'opacity')
                .attr('values', '0;0.5;0')
                .attr('dur', `${2 + Math.random() * 3}s`)
                .attr('repeatCount', 'indefinite')
                .attr('begin', `${Math.random() * 2}s`);
        });
    }

    getStarColor(message) {
        const msg = message.toLowerCase();
        if (msg.includes('release') || msg.includes('version')) return '#FFD700';
        if (msg.includes('fix')) return '#4ECDC4';
        if (msg.includes('add') || msg.includes('implement')) return '#FF6B6B';
        if (msg.includes('merge')) return '#95E1D3';
        return '#C7CEEA';
    }

    getStarSize(message) {
        const msg = message.toLowerCase();
        if (msg.includes('release') || msg.includes('version')) return 8;
        if (msg.includes('merge')) return 6;
        if (msg.includes('fix') || msg.includes('add')) return 5;
        return 4;
    }

    showTooltip(event, star) {
        const date = new Date(star.timestamp * 1000).toLocaleString();
        this.tooltip
            .style('opacity', 1)
            .style('left', `${event.pageX + 10}px`)
            .style('top', `${event.pageY - 10}px`)
            .html(`
                <strong>${star.author}</strong><br>
                ${date}<br>
                <em>${star.message}</em><br>
                <small>${star.hash.substr(0, 7)}</small>
            `);
    }

    hideTooltip() {
        this.tooltip.style('opacity', 0);
    }

    onStarClick(star) {
        // Create a pulse effect when star is clicked
        const event = d3.event || window.event;
        const pulse = this.svg.append('circle')
            .attr('cx', event.target.cx.baseVal.value)
            .attr('cy', event.target.cy.baseVal.value)
            .attr('r', 5)
            .attr('fill', 'none')
            .attr('stroke', 'white')
            .attr('stroke-width', 2)
            .attr('opacity', 1);
        
        pulse.transition()
            .duration(1000)
            .attr('r', 50)
            .attr('opacity', 0)
            .remove();
    }

    updateStats() {
        // Calculate statistics
        const totalCommits = this.commits.length;
        const uniqueAuthors = new Set(this.commits.map(c => c.author)).size;
        
        // Group by day
        const commitsByDay = {};
        this.commits.forEach(commit => {
            const date = new Date(commit.timestamp * 1000).toDateString();
            commitsByDay[date] = (commitsByDay[date] || 0) + 1;
        });
        
        const totalDays = Object.keys(commitsByDay).length;
        const biggestDay = Object.entries(commitsByDay)
            .sort((a, b) => b[1] - a[1])[0];
        
        // Update DOM
        document.getElementById('total-commits').textContent = totalCommits;
        document.getElementById('total-days').textContent = totalDays;
        document.getElementById('total-authors').textContent = uniqueAuthors;
        document.getElementById('biggest-day').textContent = 
            biggestDay ? `${biggestDay[1]} on ${new Date(biggestDay[0]).toLocaleDateString()}` : '-';
    }

    createShootingStars() {
        // Periodically create shooting stars
        setInterval(() => {
            if (Math.random() > 0.7) {
                const startX = Math.random() * this.width;
                const startY = Math.random() * this.height * 0.5;
                
                const shootingStar = document.createElement('div');
                shootingStar.className = 'shooting-star';
                shootingStar.style.left = `${startX}px`;
                shootingStar.style.top = `${startY}px`;
                shootingStar.style.animation = 'shoot 1s linear forwards';
                
                document.getElementById('cosmos').appendChild(shootingStar);
                
                setTimeout(() => shootingStar.remove(), 1000);
            }
        }, 2000);
    }

    handleResize() {
        this.width = window.innerWidth;
        this.height = window.innerHeight;
        
        // Redraw visualization
        d3.select('svg').remove();
        this.init();
    }
}

// Initialize the constellation
document.addEventListener('DOMContentLoaded', () => {
    new GitConstellation();
});