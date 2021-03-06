<template>
  <div class="graph-container" ref="container">
    <v-card v-if="highlighted" class="node-infos">
      <v-card-title primary-title>
        <div v-if="highlighted.is('node')">
          <div class="headline">{{ highlighted.id() }}</div>
          <div class="grey--text">Index: {{ highlighted.data('oid') }}</div>
          <div class="grey--text">Operation: {{ highlighted.data('op') }}</div>

          <template v-if="highlighted.data('other').op">
              <v-divider class="my-3"></v-divider>
              <div v-for="(value, key) in highlighted.data('other').op">
                <div v-if="!!value.Tensor" class="mb-3">
                  <div class="grey--text">
                    {{ key }}:
                  </div>
                  <tensor-display
                    :value="{ 'Only': value.Tensor }">
                  </tensor-display>
                </div>
                <div class="grey--text" v-else>
                  {{ key }}: {{ value }}
                </div>
              </div>
          </template>
        </div>
        <div v-else>
          <div class="headline">
            {{ hierarchy.getName(highlighted.data('source')) }}
              &rarr;
            {{ hierarchy.getName(highlighted.data('target')) }}
          </div>
          <div class="grey--text">
            Index: {{ highlighted.data('oid') }}
          </div>
          <template v-if="highlighted.data('other').fact">
            <div class="grey--text">
              Datatype: {{ highlighted.data('other').fact.datatype | typeToString }}
            </div>
            <div class="grey--text">
              Shape: {{ highlighted.data('other').fact.shape | shapeToString }}
            </div>
            <div v-if="highlighted.data('other').fact.value.Only">
              <v-divider class="my-3"></v-divider>
              <span class="grey--text">
                Value:
              </span>
              <tensor-display
                :value="highlighted.data('other').fact.value">
              </tensor-display>
            </div>
          </template>
        </div>
      </v-card-title>
    </v-card>
    <div class="graph-area" ref="area"></div>
  </div>
</template>

<style scoped>
  .graph-container {
    position: absolute;
    top: 0;
    left: 0;
    bottom: 0;
    right: 0;
    z-index: 1;
  }

  .graph-area {
    width: 100%;
    height: 100%;
    z-index: 1;
  }

  .node-infos {
    position: absolute;
    top: 98px;
    right: 28px;
    z-index: 1000;
    max-width: 550px;
    word-break: break-all;
    word-wrap: break-word;
  }
</style>

<script>
  import TensorDisplay from './TensorDisplay.vue'
  import * as helpers from './helpers'
  import { graphStyle } from './styles'

  import jquery from 'jquery'

  import cytoscape from 'cytoscape'
  import dagre from 'cytoscape-dagre'
  // import klay from 'cytoscape-klay'
  import expand from 'cytoscape-expand-collapse'
  import layout from './layout'

  // Register extensions.
  cytoscape.use(layout)
  cytoscape.use(dagre)
  // cytoscape.use(klay)
  cytoscape.use(expand, jquery)

  export default {
    props: {
      graph: Array
    },

    data: () => ({
      hierarchy: null,
      instance: null,
      parsed: null,
      highlighted: null,
    }),

    filters: {
      typeToString: helpers.typeToString,
      shapeToString: helpers.shapeToString,
    },

    components: {
      TensorDisplay
    },

    watch: {
      graph(value) {
        this.redraw()
      },

      highlighted(current, previous) {
        if (previous) {
          previous.removeClass('highlighted')
        }

        if (current) {
          console.log(current.json())
          current.addClass('highlighted')
        }
      }
    },

    methods: {
      redraw() {
        console.log('Redrawing graph with new data.')

        if (!this.graph) {
          return
        }

        if (this.instance) {
          this.instance.destroy()
        }

        this.hierarchy = new helpers.Hierarchy(this.graph.nodes)

        let nodes = this.graph.nodes
          .map(n => ({
            data: {
              id: n.name,
              oid: n.id,
              parent: this.hierarchy.getParent(n.name),
              label: this.hierarchy.getName(n.name),
              op: n.op,
              background: 'hsl(' + helpers.stringToColor(n.op_name) + ', 100%, 90%)',
              border: 'hsl(' + helpers.stringToColor(n.op_name) + ', 40%, 80%)',
              other: n,
            }
          }))

        let metanodes = this.hierarchy.getMetanodes()

        let edges = this.graph.edges
          .map(e => ({
            data: {
              id: 'e' + e.id,
              oid: e.id,
              source: this.hierarchy.getPath(e.scr_node_id),
              target: this.hierarchy.getPath(e.dst_node_id),
              label: (!!e.label) ? helpers.shapeToString(e.label) : '',
              constant: null,
              other: e,
            }
          }))

        let graph = nodes.concat(metanodes, edges)

        this.instance = cytoscape({
          container: this.$refs.area,
          elements: graph,

          wheelSensitivity: 0.1,
          autoungrabify: true,

          style: graphStyle,
        })

        this.expand = this.instance.expandCollapse({
          layoutBy: {
            name: 'tensorflow',
            randomize: false,
            fit: false,
            padding: 80,
          },
          fisheye: false,
          undoable: false,
        })

        this.expand.collapseAll()

        setTimeout(() => {
          this.instance.zoom(1.5)
          this.instance.center()
        }, 50)

        // Handle double click on metanodes.
        let clickTimer = null
        this.instance.nodes()
          .filter('[op = "Meta"]')
          .on('click', e => {
            if (!clickTimer) {
              clickTimer = setTimeout(() => clickTimer = null, 400)
            } else {
              clickTimer = null

              if (this.expand.isExpandable(e.target)) {
                this.expand.expand(e.target)
              } else {
                this.expand.collapse(e.target)
              }
            }
          })

        // Prevent selection of expanded metanodes.
        // TODO(liautaud)

        // Handle node and edge highlighting.
        this.instance
          .on('click', e => {
            if (e.target === this.instance ||
                e.target.is('node[op = "Meta"]')) {
              this.highlighted = null
            } else {
              this.highlighted = e.target
            }
          })
      }
    },

    mounted() {
      this.redraw()
    },

    destroyed() {
      this.instance.destroy()
    }
  }
</script>
