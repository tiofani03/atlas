import dagre from '@dagrejs/dagre';
import { Node, Edge, Position } from '@xyflow/react';
import { GraphNodeData } from '../../types';

export const NODE_WIDTH = 260;
export const NODE_HEIGHT = 100;

export interface LayoutOptions {
  direction?: 'LR' | 'TB';
  nodeSep?: number;
  rankSep?: number;
}

/**
 * Calculates auto-layout positions for graph nodes and edges using Dagre.
 */
export function getLayoutedElements<T extends Node<GraphNodeData>>(
  nodes: T[],
  edges: Edge[],
  options: LayoutOptions = {}
): { nodes: T[]; edges: Edge[] } {
  const direction = options.direction ?? 'LR';
  const dagreGraph = new dagre.graphlib.Graph();
  dagreGraph.setDefaultEdgeLabel(() => ({}));

  dagreGraph.setGraph({
    rankdir: direction,
    nodesep: options.nodeSep ?? 60,
    ranksep: options.rankSep ?? 90,
    marginx: 40,
    marginy: 40,
  });

  nodes.forEach((node) => {
    dagreGraph.setNode(node.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  });

  edges.forEach((edge) => {
    dagreGraph.setEdge(edge.source, edge.target);
  });

  dagre.layout(dagreGraph);

  const layoutedNodes = nodes.map((node) => {
    const nodeWithPosition = dagreGraph.node(node.id);
    // Center node relative to dagre coordinates
    const x = nodeWithPosition ? nodeWithPosition.x - NODE_WIDTH / 2 : node.position.x;
    const y = nodeWithPosition ? nodeWithPosition.y - NODE_HEIGHT / 2 : node.position.y;

    return {
      ...node,
      targetPosition: direction === 'LR' ? Position.Left : Position.Top,
      sourcePosition: direction === 'LR' ? Position.Right : Position.Bottom,
      position: { x, y },
    };
  });

  return { nodes: layoutedNodes, edges };
}
