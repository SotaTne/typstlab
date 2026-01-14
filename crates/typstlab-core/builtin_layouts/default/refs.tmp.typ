// Default layout references template
#pagebreak()

#heading(level: 1, numbering: none)[References]

{{ each paper.refs_sets |set| }}
#bibliography("../../refs/sets/{{ set }}/library.bib", style: "apa")
{{ /each }}
