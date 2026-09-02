# Validate against observations before reference simulators

Sanitized combat reports observed on an official OGame server are the strongest
validation evidence when the reporter has allowed the project to retain them.
Official rules and versioned in-game Techinfo establish individual mechanics
and constants. Dated, manual comparisons with the official in-game simulator or
third-party simulators such as OGame Tools can reveal disagreements, but their
output is supporting evidence rather than ground truth.

The test suite and CI will not scrape live report archives or simulator sites.
Public OGMem reports may be used temporarily for manual comparison and error
discovery; they enter the repository only with uploader consent and after player
names, coordinates, report keys, and other identifying fields are removed.
