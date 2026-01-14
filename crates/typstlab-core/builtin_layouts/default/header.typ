// Default layout header
#set page(
  header: context [
    #set text(size: 9pt)
    #grid(
      columns: (1fr, 1fr),
      align(left)[_Typstlab Document_],
      align(right)[Page #counter(page).display("1 / 1", both: true)],
    )
    #line(length: 100%, stroke: 0.5pt)
  ],
  margin: (top: 2cm, bottom: 2cm, left: 2cm, right: 2cm),
)

#set par(justify: true, leading: 0.65em)
#set text(font: "New Computer Modern", size: 11pt)
