# Names and titles

Names are the text that identifies a shape: a component's or state's name, a package's name in its tab,
the name beside an entry point. The diagram title is a caption. Labels on lines are in [Labels](labels.md).

## Centre a classifier's name, keyword above

**Rule.** A component, class, interface or other classifier shows its name centred in bold, with any
keyword or stereotype centred in plain text above it, in guillemets. This includes a composite
component, whose name is centred in its header.

**Why.** UML asks for it, and a centred name marks the box as one unit.

**Source.** UML 2.5.1, 9.2.4.1, p. 101: "The name of the Classifier should be centered in boldface",
with keywords "centered in plain face within guillemets above the Classifier name".

**Status:** enforced in component and package diagrams.

## Put a package's name in its tab when it shows its contents

**Rule.** A package that shows its members carries its name, left-aligned, in the tab. A package
without visible contents has a blank tab and its name in the body.

**Why.** UML specifies both cases. Readers find the name in the same place in every package.

```drumlin
diagram package "Names in tabs"

package Core {
  class Order
  class Invoice
}
package Util
Core ..> Util : <<import>>
```

**Source.** UML 2.5.1, 12.2.4, p. 248.

**Status:** enforced, in package diagrams and for packages inside component diagrams.

## Keep lines off names

**Rule.** No line crosses a name, a keyword, a composite state's internal activities, or a package tab.

**Why.** A line through text hides both. A line through a tab reads as entering the tab, which means
nothing.

Bad: the arrow pierces `Util`'s tab to reach the body below it.

<figure class="drumlin-figure">
<svg xmlns="http://www.w3.org/2000/svg" class="drumlin pens" viewBox="120 0 184 180" width="184" height="180">
<style>
.drumlin.pens text { font-family: Helvetica, Arial, "Liberation Sans", sans-serif; fill: #22201c; }
.drumlin.pens .name { font-size: 13px; font-weight: 600; }
.drumlin.pens .small { font-size: 11px; fill: #22201c; }
.drumlin.pens .shape { fill: #fff; stroke: #22201c; stroke-width: 1.25; stroke-linejoin: miter; }
.drumlin.pens .edge { fill: none; stroke: #22201c; stroke-width: 1.25; stroke-linecap: butt; stroke-linejoin: miter; }
.drumlin.pens .dashed { stroke-dasharray: 7 3; }
</style>
<rect x="120" y="0" width="184" height="180" fill="#fff"/>
<path d="M136 96 L136 72 L196 72 L196 96" class="shape"/>
<rect x="136" y="96" width="152" height="72" class="shape"/>
<text x="212" y="136.5" class="name" text-anchor="middle">Util</text>
<rect x="136" y="8" width="112" height="40" class="shape"/>
<text x="192.0" y="32.5" class="name" text-anchor="middle">App</text>
<path d="M160 48 L160 96" class="edge"/>
<path d="M155.5 86 L160 96 L164.5 86" class="edge"/>
</svg>
</figure>

**Source.** [Keep lines out of shapes and names](lines.md#keep-lines-out-of-shapes-and-names).

**Status:** enforced. Composite names, state headers and tabs are obstacles for every line that enters
their container. The [levers study](levers.md#titles) found that, as obstacles, they cost no bend.

## Let arrows end on a package's tab

**Rule.** A relation arriving at a package from above may end on the top of the tab, when that is
where its straight line meets the package.

**Why.** The tab is part of the package symbol, so a line ending there points at the package. If the
top side started right of the tab, a line that would arrive over the tab would jog sideways instead.

**Source.** UML 2.5.1, 12.2.4, p. 248 (the tab is part of the symbol); [Layout levers](levers.md#titles).

**Status:** enforced for packages without visible contents, whose tab is blank (`tab_takes_ends` in the
package routing). An end on the tab stays 8px from its corners and 16px from the step down to the body.
The Relations and Package merge examples show it. A package showing its contents keeps its name in the
tab, so its top side still starts right of the tab.

## Let a name give way to a line

**Rule.** Where the notation leaves the name's position free, move the name aside instead of bending a
line. A composite state's name may sit left or right in its name compartment, or in a UML name tab,
when an entry point or a straight line wants the middle. A classifier's name stays centred, because
9.2.4.1 asks for it.

**Why.** The name can move without loss. The line cannot bend without cost: an entry point that slides
off a centred name makes the transition into it jog.

**Source.** UML 2.5.1, 14.2.4.4, p. 319-320, which allows an optional name tab "resting on the outside
of the top side of a State"; [Layout levers](levers.md#titles).

**Status:** enforced for entry points on the top border of a composite state in a downward flow (`name_place` in the state layout): when the entry point that lines up with its target meets the
centred name, the name moves beside the entry point, 8px clear of its circle, on the side away from the
middle first, keeping 12px from the state's sides. Only when neither side has room does the entry point
slide off the name instead. In the Entry and exit points example the chain `Idle`, `again`, `Prompt`
runs straight this way. The UML name tab is not used.

## Keep connection point names beside their symbols

**Rule.** An entry or exit point's name sits next to its circle, clear of the state's own name, and
the state's name moves aside for the entry point, or, where it has no room, the entry point slides along
the border off that name.

**Why.** Two names overlapping cannot be read, and a connection point name far from its circle is
easily taken for the state's.

**Source.** UML 2.5.1, 14.2.4.6, p. 327-328 (entry and exit points as circles on the border, "with the name associated with it").

**Status:** enforced (`name_place` and `clear_of_name` in the state layout).

## Caption the diagram, do not frame it

**Rule.** A `diagram kind "Title"` title is a caption above the drawing, outside its contents area.
There is no UML frame.

**Why.** A frame adds a box around every drawing and competes with the content. UML makes the frame
optional, and drops its heading with it.

**Source.** UML 2.5.1, Annex A; a design decision of this project.

**Status:** enforced, by `core::svg::document`.
