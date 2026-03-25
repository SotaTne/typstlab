// Default template.typ (Static)
#let project(
  title: "",
  authors: (),
  date: none,
  language: "en",
  body
) = {
  // Set document metadata
  set document(title: title, author: authors.map(a => a.name))
  set text(lang: language, size: 11pt, font: "New Computer Modern")
  
  // Page settings
  set page(
    paper: "a4",
    margin: (auto),
    header: context [
      #set text(size: 9pt)
      #grid(
        columns: (1fr, 1fr),
        align(left)[_#title _],
        align(right)[Page #counter(page).display("1 / 1", both: true)],
      )
      #line(length: 100%, stroke: 0.5pt)
    ],
    numbering: "1",
  )

  // Paragraph settings
  set par(justify: true, leading: 0.65em)

  // Title
  align(center)[
    #block(text(weight: "bold", size: 1.5em, title))
    #v(1em)
    #grid(
      columns: (1fr,) * calc.min(3, authors.len()),
      column-gap: 1em,
      row-gap: 1.5em,
      ..authors.map(author => [
        #text(weight: "bold", author.name) \
        #author.affiliation \
        #link("mailto:" + author.email)
      ]),
    )
    #v(1em)
    #date
  ]
  
  #v(2em)
  
  body
}
