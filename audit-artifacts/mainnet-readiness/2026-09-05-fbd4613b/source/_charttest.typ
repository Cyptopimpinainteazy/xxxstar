#set page(width: 18cm, height: 24cm, margin: 1cm)
#import "charts.typ": *
#import "data.typ": *

= severity chart
#severity-bar-chart(sev-counts)

= subsystem score chart
#subsystem-score-chart(subsystem-scores)

= feature status chart
#feature-status-chart(feature-status-counts)

Overall score: #overall-score
