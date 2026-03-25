#import "template.typ": *

#show: project.with(
  title: "{{ paper.title }}",
  authors: (
    {{ each paper.authors |author| }}
    (
      name: "{{ author.name }}",
      email: "{{ author.email }}",
      affiliation: "{{ author.affiliation }}",
    ),
    {{ /each }}
  ),
  date: "{{ paper.date }}",
  language: "{{ paper.language }}",
)

= Introduction
This is a new paper created with typstlab.

== Section 1
Hello, Typst!
