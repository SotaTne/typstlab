// Minimal layout references template
#pagebreak()

#heading(level: 1)[References]

{{ each refs.sets |set| }}
#bibliography("../../refs/sets/{{ set }}/library.bib")
{{ /each }}
