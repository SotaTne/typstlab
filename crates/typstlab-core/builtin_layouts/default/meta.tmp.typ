// Default layout meta template
#set text(lang: "{{ paper.language }}")
#set text(size: 12pt)
#set page(numbering: "1")

#align(center)[
  #text(size: 18pt, weight: "bold")[{{ paper.title }}]
]

#v(1em)

#align(center)[
  {{ each paper.authors |author| }}
  #text(size: 11pt)[
    *{{ author.name }}* \
    #text(style: "italic", size: 10pt)[{{ author.affiliation }}] \
    #text(size: 10pt)[{{ author.email }}]
  ]
  #v(0.5em)
  {{ /each }}
]

#v(1em)

#align(center)[
  #text(size: 10pt, style: "italic")[{{ paper.date }}]
]

#v(2em)
