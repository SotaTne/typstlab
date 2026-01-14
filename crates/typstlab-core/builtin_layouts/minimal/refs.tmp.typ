// Minimal layout references template
#pagebreak()

#heading(level: 1)[References]

{{ each paper.refs_sets |set| }}
#bibliography("../../refs/sets/{{ set }}/library.bib")
{{ /each }}
