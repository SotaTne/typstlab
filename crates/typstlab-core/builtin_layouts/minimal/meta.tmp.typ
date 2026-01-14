// Minimal layout meta template
#set text(lang: "{{ paper.language }}")

#align(center)[
  #text(size: 16pt, weight: "bold")[{{ paper.title }}]
]

#v(0.5em)

#align(center)[
  #text(size: 10pt, style: "italic")[{{ paper.date }}]
]

#v(1em)
